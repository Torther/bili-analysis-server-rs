use crate::mirror_cdn::upgrade_cdn_hostname;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::Duration;

const XOR_CODE: u128 = 23442827791579;
const MAX_AID: u128 = 1 << 51;
const BASE: u128 = 58;
const BV_DATA: &str = "FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .pool_max_idle_per_host(10)
        .build()
        .expect("failed to build reqwest client")
});

static BV_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"BV[0-9a-zA-Z]{10}").unwrap());
static AV_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:av|AV)(\d+)").unwrap());
static P_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]p=(\d+)").unwrap());
static LIVE_ROOM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"live\.bilibili\.com/(?:blanc/)?(\d+)").unwrap());

#[derive(Deserialize)]
struct PageItem {
    cid: u64,
}

#[derive(Deserialize)]
struct PageListResponse {
    data: Vec<PageItem>,
}

#[derive(Deserialize)]
struct DurlItem {
    url: String,
}

#[derive(Deserialize)]
struct PlayUrlData {
    durl: Vec<DurlItem>,
}

#[derive(Deserialize)]
struct PlayUrlResponse {
    data: PlayUrlData,
}

#[derive(Deserialize)]
struct LiveCodecInfo {
    base_url: String,
    url_info: Option<Vec<LiveUrlInfo>>,
}

#[derive(Deserialize)]
struct LiveUrlInfo {
    host: String,
    extra: String,
}

#[derive(Deserialize)]
struct LiveFormat {
    codec: Option<Vec<LiveCodecInfo>>,
}

#[derive(Deserialize)]
struct LiveStream {
    format: Option<Vec<LiveFormat>>,
}

#[derive(Deserialize)]
struct LivePlayUrl {
    stream: Option<Vec<LiveStream>>,
}

#[derive(Deserialize)]
struct LivePlayUrlInfo {
    playurl: LivePlayUrl,
}

#[derive(Deserialize)]
struct LiveRoomData {
    playurl_info: Option<LivePlayUrlInfo>,
}

#[derive(Deserialize)]
struct LiveRoomResponse {
    data: Option<LiveRoomData>,
}

fn av2bv(av: &str) -> Result<String, String> {
    let aid: u128 = av
        .strip_prefix("av")
        .unwrap_or(av)
        .parse()
        .map_err(|_| format!("invalid AV number: {}", av))?;
    let mut bytes = vec![
        'B', 'V', '1', '0', '0', '0', '0', '0', '0', '0', '0', '0',
    ];
    let mut bv_index = bytes.len() - 1;
    let mut tmp = (MAX_AID | aid) ^ XOR_CODE;
    while tmp > 0 {
        bytes[bv_index] = BV_DATA
            .chars()
            .nth((tmp % BASE) as usize)
            .ok_or("av2bv: index out of bounds")?;
        tmp /= BASE;
        if bv_index == 0 {
            break;
        }
        bv_index -= 1;
    }
    bytes.swap(3, 9);
    bytes.swap(4, 7);
    Ok(bytes.into_iter().collect())
}

fn is_bilibili_domain(hostname: &str) -> bool {
    hostname == "bilibili.com"
        || hostname.ends_with(".bilibili.com")
}

fn validate_bilibili_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or("URL has no hostname")?;
    if !is_bilibili_domain(host) {
        return Err(format!("URL hostname '{}' is not bilibili.com", host));
    }
    Ok(())
}

fn check_api_response(body: &serde_json::Value) -> Result<(), String> {
    if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
        if code != 0 {
            let msg = body
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(format!("API error (code={}): {}", code, msg));
        }
    }
    Ok(())
}

async fn api_get_json(url: &str, params: &[(&str, &str)], referer: &str) -> Result<serde_json::Value, String> {
    let res = CLIENT
        .get(url)
        .query(params)
        .header("Referer", referer)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("request to {} failed: {}", url, e))?;

    if !res.status().is_success() {
        return Err(format!("request to {} failed, status: {}", url, res.status()));
    }

    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("invalid JSON from {}: {}", url, e))?;
    check_api_response(&json)?;
    Ok(json)
}

struct VideoInfo {
    bvid: String,
    page: usize,
}

fn extract_video_info(url: &str) -> Result<VideoInfo, String> {
    let bv_match = BV_RE.find(url).map(|m| m.as_str().to_string());
    let av_match = AV_RE.captures(url).and_then(|c| c.get(1)).map(|m| m.as_str());

    let bvid = match bv_match {
        Some(bv) => bv,
        None => match av_match {
            Some(av) => av2bv(av)?,
            None => return Err("Cannot find BV/AV ID".to_string()),
        },
    };

    let page = P_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .map(|p| p.max(1))
        .unwrap_or(1);

    Ok(VideoInfo { bvid, page })
}

fn extract_live_room_id(url: &str) -> Option<String> {
    LIVE_ROOM_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn is_bare_id(input: &str) -> bool {
    if input.starts_with("BV") || input.starts_with("bv") {
        return true;
    }
    let lower = input.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("av") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

pub fn extract_cache_key(target_url: &str) -> Result<String, String> {
    let trimmed = target_url.trim();

    if is_bare_id(trimmed) {
        let video_info = extract_video_info(trimmed)?;
        return Ok(format!("vid:{}:{}", video_info.bvid, video_info.page));
    }

    validate_bilibili_url(trimmed)?;

    if trimmed.contains("live.bilibili.com") {
        let room_id = extract_live_room_id(trimmed)
            .ok_or("Cannot find live room ID")?;
        return Ok(format!("live:{}", room_id));
    }

    let video_info = extract_video_info(trimmed)?;
    Ok(format!("vid:{}:{}", video_info.bvid, video_info.page))
}

async fn fetch_video_cid(bvid: &str) -> Result<Vec<PageItem>, String> {
    let json = api_get_json(
        &format!("https://api.bilibili.com/x/player/pagelist?bvid={}", bvid),
        &[],
        "https://www.bilibili.com/",
    ).await?;

    let parsed: PageListResponse =
        serde_json::from_value(json).map_err(|e| format!("Invalid pagelist structure: {}", e))?;

    Ok(parsed.data)
}

async fn fetch_video_url(bvid: &str, cid: u64) -> Result<String, String> {
    let json = api_get_json(
        "https://api.bilibili.com/x/player/playurl",
        &[
            ("bvid", bvid),
            ("cid", &cid.to_string()),
            ("qn", "116"),
            ("otype", "json"),
            ("platform", "html5"),
            ("high_quality", "1"),
        ],
        "https://www.bilibili.com/",
    ).await?;

    let parsed: PlayUrlResponse =
        serde_json::from_value(json).map_err(|e| format!("Invalid playurl structure: {}", e))?;

    parsed
        .data
        .durl
        .first()
        .map(|d| d.url.clone())
        .ok_or_else(|| "Cannot get video URL from response".to_string())
}

async fn fetch_live_url(room_id: &str) -> Result<String, String> {
    let json = api_get_json(
        "https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo",
        &[
            ("room_id", room_id),
            ("protocol", "0,1"),
            ("format", "0,1,2"),
            ("codec", "0,1"),
            ("qn", "10000"),
            ("platform", "web"),
            ("ptype", "8"),
            ("dolby", "5"),
            ("panorama", "1"),
        ],
        "https://live.bilibili.com/",
    ).await?;

    let parsed: LiveRoomResponse =
        serde_json::from_value(json).map_err(|e| format!("Invalid live response structure: {}", e))?;

    let playurl_info = parsed
        .data
        .and_then(|d| d.playurl_info)
        .ok_or("Cannot get live playurl info")?;

    let streams = playurl_info.playurl.stream.unwrap_or_default();

    // Bilibili live API returns streams in priority order.
    // stream[0] = FLV, stream[1] = HLS (generally more compatible).
    // Within each stream, format[0] = standard, format[1] = HEVC (better compression).
    // codec[0] is the primary codec for that format.
    // We prefer HLS + HEVC for quality, falling back to FLV + standard.
    for stream_idx in [1usize, 0] {
        let stream = match streams.get(stream_idx) {
            Some(s) => s,
            None => continue,
        };
        let formats = match &stream.format {
            Some(f) => f,
            None => continue,
        };
        for format_idx in [1usize, 0] {
            let fmt = match formats.get(format_idx) {
                Some(f) => f,
                None => continue,
            };
            let codec = match &fmt.codec {
                Some(c) => &c[..],
                None => continue,
            };
            let codec_info = match codec.first() {
                Some(c) => c,
                None => continue,
            };
            let url_info = match &codec_info.url_info {
                Some(u) => &u[..],
                None => continue,
            };
            let info = match url_info.first() {
                Some(i) => i,
                None => continue,
            };
            return Ok(format!("{}{}{}", info.host, codec_info.base_url, info.extra));
        }
    }

    Err("Cannot get live stream URL from response".to_string())
}

async fn resolve_video(input: &str) -> Result<String, String> {
    let video_info = extract_video_info(input)?;
    let page_list = fetch_video_cid(&video_info.bvid).await?;
    let page_item = page_list
        .get(video_info.page.saturating_sub(1))
        .ok_or(format!("Page {} not found", video_info.page))?;
    let raw_url = fetch_video_url(&video_info.bvid, page_item.cid).await?;
    Ok(upgrade_cdn_hostname(&raw_url))
}

pub async fn resolve_raw_url(target_url: &str) -> Result<String, String> {
    let trimmed = target_url.trim();

    if is_bare_id(trimmed) {
        return resolve_video(trimmed).await;
    }

    validate_bilibili_url(trimmed)?;

    if trimmed.contains("live.bilibili.com") {
        let room_id = extract_live_room_id(trimmed)
            .ok_or("Cannot find live room ID")?;
        let raw_url = fetch_live_url(&room_id).await?;
        return Ok(upgrade_cdn_hostname(&raw_url));
    }

    resolve_video(trimmed).await
}
