use dashmap::DashMap;
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::LazyLock;
use std::task::{Context, Poll};
use tokio::sync::watch;

type SharedResult = Result<String, String>;

static IN_FLIGHT: LazyLock<DashMap<String, watch::Sender<Option<SharedResult>>>> =
    LazyLock::new(DashMap::new);

struct WatchFuture {
    rx: watch::Receiver<Option<SharedResult>>,
}

impl Future for WatchFuture {
    type Output = SharedResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.rx.has_changed() {
            Ok(true) => {
                let val = self.rx.borrow_and_update().clone();
                match val {
                    Some(result) => Poll::Ready(result),
                    None => Poll::Pending,
                }
            }
            Ok(false) => {
                let changed = pin!(self.rx.changed());
                let _ = changed.poll(cx);
                Poll::Pending
            }
            Err(_) => Poll::Ready(Err("fetch task panicked".into())),
        }
    }
}

pub async fn dedup_resolve(
    target_url: &str,
) -> SharedResult {
    let cache_key = crate::bilibili::extract_cache_key(target_url)?;

    if let Some(cached) = crate::cache::CACHE.get(&cache_key) {
        return Ok(cached);
    }

    let rx = {
        let entry = IN_FLIGHT.entry(cache_key.clone()).or_insert_with(|| {
            let (tx, _) = watch::channel(None);
            tx
        });
        entry.value().subscribe()
    };

    let has_sender = IN_FLIGHT.get(&cache_key).map(|e| !e.is_closed()).unwrap_or(false);

    if has_sender {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            WatchFuture { rx },
        )
        .await;

        match result {
            Ok(Ok(url)) => return Ok(url),
            Ok(Err(e)) => return Err(e),
            Err(_) => {}
        }
    }

    let (tx, _rx) = watch::channel(None);
    IN_FLIGHT.insert(cache_key.clone(), tx.clone());

    let result = crate::bilibili::resolve_raw_url(target_url).await;

    if let Ok(ref url) = result {
        crate::cache::CACHE.insert(cache_key.clone(), url.clone());
    }

    let _ = tx.send(Some(result.clone()));
    IN_FLIGHT.remove(&cache_key);

    result
}
