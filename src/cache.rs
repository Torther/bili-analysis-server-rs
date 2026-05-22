use moka::sync::Cache;
use std::sync::LazyLock;
use std::time::Duration;

const CACHE_TTL_SECS: u64 = 600;

pub static CACHE: LazyLock<Cache<String, String>> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
        .build()
});
