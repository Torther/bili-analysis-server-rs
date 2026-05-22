use crate::mirror_cdn::upgrade_cdn_hostname;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

const XOR_CODE: u128 = 23442827791579;
const MAX_AID: u128 = 1 << 51;
const BASE: u128 = 58;
const BV_DATA: &str = "FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";

static BV_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"BV[0-9a-zA-Z]+").unwrap());
static AV_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"av(\d+)").unwrap());
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

fn av2bv(av: &str) -> String {
    let aid: u128 = av
        .strip_prefix("av")
        .unwrap_or(av)
        .parse()
        .unwrap_or(0);
    let mut bytes = vec![
        'B', 'V', '1', '0', '0', '0', '0', '0', '0', '0', '0', '0',
    ];
    let mut bv_index = bytes.len() - 1;
    let mut tmp = (MAX_AID | aid) ^ XOR_CODE;
    while tmp > 0 {
        bytes[bv_index] = BV_DATA.chars().nth((tmp % BASE) as usize).unwrap();
        tmp /= BASE;
        bv_index -= 1;
    }
    bytes.swap(3, 9);
    bytes.swap(4, 7);
    bytes.into_iter().collect()
}

struct VideoInfo {
    bvid: String,
    page: usize,
}

fn extract_video_info(url: &str) -> Option<VideoInfo> {
    let bv_match = BV_RE.find(url).map(|m| m.as_str().to_string());
    let av_match = AV_RE.captures(url).and_then(|c| c.get(1)).map(|m| m.as_str());

    let bvid = bv_match.or_else(|| av_match.map(av2bv))?;
    let page = P_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .map(|p| p.max(1))
        .unwrap_or(1);

    Some(VideoInfo { bvid, page })
}

fn extract_live_room_id(url: &str) -> Option<String> {
    LIVE_ROOM_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

async fn fetch_video_cid(bvid: &str) -> Result<Vec<PageItem>, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "https://api.bilibili.com/x/player/pagelist?bvid={}",
            bvid
        ))
        .send()
        .await
        .map_err(|e| format!("CID request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("CID request failed, status: {}", res.status()));
    }

    let json: PageListResponse = res
        .json()
        .await
        .map_err(|e| format!("Invalid pagelist response: {}", e))?;

    Ok(json.data)
}

async fn fetch_video_url(bvid: &str, cid: u64) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.bilibili.com/x/player/playurl")
        .query(&[
            ("bvid", bvid),
            ("cid", &cid.to_string()),
            ("qn", "116"),
            ("type", ""),
            ("otype", "json"),
            ("platform", "html5"),
            ("high_quality", "1"),
        ])
        .header("Referer", "https://www.bilibili.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("Playurl request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Playurl request failed, status: {}", res.status()));
    }

    let json: PlayUrlResponse = res
        .json()
        .await
        .map_err(|e| format!("Invalid playurl response: {}", e))?;

    json.data
        .durl
        .first()
        .map(|d| d.url.clone())
        .ok_or_else(|| "Cannot get video URL from response".to_string())
}

async fn fetch_live_url(room_id: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo")
        .query(&[
            ("room_id", room_id),
            ("protocol", "0,1"),
            ("format", "0,1,2"),
            ("codec", "0,1"),
            ("qn", "10000"),
            ("platform", "web"),
            ("ptype", "8"),
            ("dolby", "5"),
            ("panorama", "1"),
        ])
        .header("Referer", "https://live.bilibili.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("Live request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Live request failed, status: {}", res.status()));
    }

    let json: LiveRoomResponse = res
        .json()
        .await
        .map_err(|e| format!("Invalid live response: {}", e))?;

    let playurl_info = json
        .data
        .and_then(|d| d.playurl_info)
        .ok_or("Cannot get live playurl info")?;

    let streams = playurl_info.playurl.stream.unwrap_or_default();

    for stream_index in [1usize, 0] {
        let stream = match streams.get(stream_index) {
            Some(s) => s,
            None => continue,
        };
        let formats = match &stream.format {
            Some(f) => f,
            None => continue,
        };
        for format_index in [1usize, 0] {
            let fmt = match formats.get(format_index) {
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

pub async fn resolve_raw_url(target_url: &str) -> Result<String, String> {
    if target_url.contains("live.bilibili.com") {
        let room_id = extract_live_room_id(target_url)
            .ok_or("Cannot find live room ID")?;
        let raw_url = fetch_live_url(&room_id).await?;
        return Ok(upgrade_cdn_hostname(&raw_url));
    }

    if target_url.contains("bilibili.com") {
        let video_info =
            extract_video_info(target_url).ok_or("Cannot find BV/AV ID in URL")?;
        let page_list = fetch_video_cid(&video_info.bvid).await?;
        let page_item = page_list
            .get(video_info.page.saturating_sub(1))
            .ok_or(format!("Page {} not found", video_info.page))?;
        let raw_url = fetch_video_url(&video_info.bvid, page_item.cid).await?;
        return Ok(upgrade_cdn_hostname(&raw_url));
    }

    Err("Unsupported URL format".to_string())
}
