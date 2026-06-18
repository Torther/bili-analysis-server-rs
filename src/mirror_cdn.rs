use regex::Regex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;
use url::Url;

const MIRROR_CDN_CHINA: &[&str] = &[
    "upos-sz-mirrorali.bilivideo.com",
    "upos-sz-mirroralib.bilivideo.com",
    "upos-sz-mirroralio1.bilivideo.com",
    "upos-sz-mirrorbd.bilivideo.com",
    "upos-sz-mirrorcos.bilivideo.com",
    "upos-sz-mirrorcosb.bilivideo.com",
    "upos-sz-mirrorcoso1.bilivideo.com",
    "upos-sz-mirrorhw.bilivideo.com",
    "upos-sz-mirrorhwb.bilivideo.com",
    "upos-sz-mirrorhwo1.bilivideo.com",
    "upos-sz-mirror08c.bilivideo.com",
    "upos-sz-mirror08h.bilivideo.com",
    "upos-sz-mirror08ct.bilivideo.com",
];

const PROXY_TF: &str = "proxy-tf-all-ws.bilivideo.com";

static MIRROR_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^upos-(sz|hz|bstar)-mirror([0-9a-z]+)\.(bilivideo\.com|akamaized\.net)$").unwrap()
});
static MCDN_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^/v1/resource").unwrap()
});
static PROXY_TF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(upos|proxy).*-tf-.*\.bilivideo\.com$").unwrap()
});
static OVERSEAS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"mirror[a-z0-9]*ov\.").unwrap()
});

static INDEX_CHINA: AtomicUsize = AtomicUsize::new(0);

fn is_mirror_cdn(hostname: &str) -> bool {
    MIRROR_HOST_RE.is_match(hostname)
}

fn is_overseas_cdn(hostname: &str) -> bool {
    OVERSEAS_RE.is_match(hostname)
        || hostname.contains("mirrorcf")
        || hostname.contains("bstar")
        || hostname.ends_with("akamaized.net")
}

fn is_proxy_tf(hostname: &str) -> bool {
    PROXY_TF_RE.is_match(hostname)
}

fn is_mcdn_ip_port(hostname: &str) -> bool {
    hostname
        .split('.')
        .all(|part| part.parse::<u8>().is_ok())
        && hostname.matches('.').count() == 3
}

fn is_mcdn_domain(hostname: &str) -> bool {
    hostname.contains("mcdn.bilivideo")
}

fn pick_mirror_china() -> &'static str {
    let idx = INDEX_CHINA.fetch_add(1, Ordering::Relaxed) % MIRROR_CDN_CHINA.len();
    MIRROR_CDN_CHINA[idx]
}

pub fn upgrade_cdn_hostname(raw_url: &str) -> String {
    let mut url = match Url::parse(raw_url) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("[mirror-cdn] Failed to parse URL: {}", e);
            return raw_url.to_string();
        }
    };

    let hostname = url.host_str().unwrap_or("").to_string();
    let pathname = url.path().to_string();

    if is_proxy_tf(&hostname) {
        return url.to_string();
    }

    if is_overseas_cdn(&hostname) {
        let _ = url.set_host(Some(pick_mirror_china()));
        return url.to_string();
    }

    if is_mirror_cdn(&hostname) {
        if url.query_pairs().any(|(k, v)| k == "os" && v == "mcdn") {
            let _ = url.set_host(Some(pick_mirror_china()));
            return url.to_string();
        }
        return url.to_string();
    }

    if is_mcdn_ip_port(&hostname) || (is_mcdn_domain(&hostname) && MCDN_PATH_RE.is_match(&pathname)) {
        let _ = url.set_host(Some(PROXY_TF));
        return url.to_string();
    }

    let _ = url.set_host(Some(pick_mirror_china()));
    url.to_string()
}
