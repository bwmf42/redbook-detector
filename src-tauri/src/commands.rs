use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Serialize, Deserialize)]
pub struct XhsResult {
    pub success: bool,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    percent: i32,
    status: String,
}

fn emit_progress(window: &tauri::Window, percent: i32, status: &str) {
    let _ = window.emit(
        "download-progress",
        ProgressPayload {
            percent,
            status: status.to_string(),
        },
    );
}

fn sanitize_filename(s: &str) -> String {
    let s = s
        .replace('/', "_")
        .replace('\\', "_")
        .replace(':', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
        .replace('\n', " ")
        .replace('\r', " ");
    let s = s.trim().trim_matches('.').trim_matches(' ');
    if s.is_empty() {
        "video".to_string()
    } else {
        s.chars().take(40).collect()
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() > max {
        s.chars().take(max).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

/// HTML 实体解码，循环两次处理双重编码
fn html_decode(s: &str) -> String {
    let mut result = s.to_string();
    for _ in 0..2 {
        result = result
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&#34;", "\"")
            .replace("&#39;", "'")
            .replace("&#60;", "<")
            .replace("&#62;", ">")
            .replace("&#x22;", "\"")
            .replace("&#x27;", "'");
    }
    result
}

#[tauri::command]
pub async fn select_folder() -> Option<String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("选择保存文件夹")
        .pick_folder()
        .await;
    folder.map(|f| f.path().to_string_lossy().into_owned())
}

#[tauri::command]
pub fn download_xhs(url: String, save_dir: Option<String>, window: tauri::Window) -> XhsResult {
    let save_path = save_dir.map(PathBuf::from).unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Desktop"))
            .unwrap_or_else(|_| PathBuf::from("."))
    });

    emit_progress(&window, 0, "解析链接...");
    let resolved = match resolve_url(&url) {
        Ok(u) => u,
        Err(e) => {
            emit_progress(&window, 0, "idle");
            return XhsResult {
                success: false,
                message: format!("链接解析失败: {}", e),
                path: None,
            };
        }
    };

    emit_progress(&window, 5, "提取视频信息...");
    let (video_url, author, title, likes) = match extract_note_info(&resolved) {
        Ok(info) => info,
        Err(e) => {
            emit_progress(&window, 0, "idle");
            return XhsResult {
                success: false,
                message: format!("提取信息失败: {}", e),
                path: None,
            };
        }
    };

    // 构建文件名: 博主_标题_赞数.mp4
    let author_clean = sanitize_filename(&author);
    let title_clean = sanitize_filename(&title);
    let likes_clean = sanitize_filename(&likes);
    let filename = format!("{}_{}_{}.mp4", author_clean, title_clean, likes_clean);
    let filepath = save_path.join(&filename);

    // 如果已存在，加序号
    let filepath = if filepath.exists() {
        let stem = filepath.file_stem().unwrap_or_default().to_string_lossy();
        let mut i = 1;
        let mut candidate = save_path.join(format!("{}_{}.mp4", stem, i));
        while candidate.exists() {
            i += 1;
            candidate = save_path.join(format!("{}_{}.mp4", stem, i));
        }
        candidate
    } else {
        filepath
    };

    emit_progress(&window, 10, "开始下载...");
    match download_file(&video_url, &filepath, &window) {
        Ok(size) => {
            emit_progress(&window, 100, "完成");
            XhsResult {
                success: true,
                message: format!("下载成功 ({:.1} MB)", size as f64 / 1_048_576.0),
                path: Some(filepath.to_string_lossy().into_owned()),
            }
        }
        Err(e) => {
            emit_progress(&window, 0, "idle");
            XhsResult {
                success: false,
                message: format!("下载失败: {}", e),
                path: None,
            }
        }
    }
}

#[tauri::command]
pub fn open_in_finder(path: String) {
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
}

fn resolve_url(url: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let resp = agent
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .call()
        .map_err(|e| e.to_string())?;
    Ok(resp.get_url().to_string())
}

fn extract_note_info(page_url: &str) -> Result<(String, String, String, String), String> {
    let body = ureq::get(page_url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;

    // 优先从 __INITIAL_STATE__ 解析（最完整、最可靠）
    if let Some(info) = extract_from_initial_state(&body) {
        return Ok(info);
    }

    // 回退：正则提取
    extract_from_regex(&body)
}

/// 从 window.__INITIAL_STATE__ 解析 note 数据
fn extract_from_initial_state(body: &str) -> Option<(String, String, String, String)> {
    let start = body.find("window.__INITIAL_STATE__=")?;
    let json_start = start + "window.__INITIAL_STATE__=".len();
    let end = body[json_start..].find("</script>")?;
    let json_str = &body[json_start..json_start + end];

    // 预处理非法 JSON 值
    let cleaned = json_str
        .replace(":undefined", ":null")
        .replace(":NaN", ":null")
        .replace(":Infinity", ":null")
        .replace(":-Infinity", ":null");

    let data: Value = serde_json::from_str(&cleaned).ok()?;

    // 定位 note.noteDetailMap.{noteId}.note
    let note = data.get("note")?;
    let note_detail_map = note.get("noteDetailMap")?;
    let detail = note_detail_map.as_object()?.values().next()?;
    let note_obj = detail.get("note")?;

    let title = note_obj
        .get("title")?
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    let title = if title.is_empty() {
        note_obj
            .get("desc")?
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        title
    };

    let user = note_obj.get("user")?;
    let author = user
        .get("nickname")?
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let interact = note_obj.get("interactInfo")?;
    let likes = interact
        .get("likedCount")?
        .as_u64()
        .map(|n| n.to_string())
        .or_else(|| interact.get("likedCount")?.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "0".to_string());

    // 视频 URL: video.media.stream.h264[0].masterUrl
    let video_url = note_obj
        .get("video")?
        .get("media")?
        .get("stream")?
        .get("h264")?
        .as_array()?
        .first()?
        .get("masterUrl")?
        .as_str()?
        .to_string();

    Some((video_url, author, title, likes))
}

/// 正则回退提取
fn extract_from_regex(body: &str) -> Result<(String, String, String, String), String> {
    // 视频 URL
    let video_re =
        Regex::new(r#"https://sns-video[^"'\s<>]+(?:mp4|m3u8)[^"'\s<>]*"#).map_err(|e| e.to_string())?;
    let video_url = video_re
        .find(body)
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "未在页面中找到视频链接".to_string())?;

    let title = extract_title_regex(body);
    let author = extract_author_regex(body);
    let likes = extract_likes_regex(body);

    Ok((video_url, author, title, likes))
}

fn extract_title_regex(body: &str) -> String {
    // 优先 og:title
    if let Some(re) = Regex::new(r#"<meta[^>]*property="og:title"[^>]*content="([^"]*)""#).ok() {
        if let Some(caps) = re.captures(body) {
            let t = html_decode(caps.get(1).map(|m| m.as_str().trim()).unwrap_or(""));
            if !t.is_empty() && t != "小红书" {
                return truncate(&t, 20);
            }
        }
    }
    // fallback: <title>
    if let Some(re) = Regex::new(r#"<title>([^<]*)</title>"#).ok() {
        if let Some(caps) = re.captures(body) {
            let t = html_decode(caps.get(1).map(|m| m.as_str().trim()).unwrap_or(""));
            if !t.is_empty() && t != "小红书" {
                return truncate(&t, 20);
            }
        }
    }
    // fallback: desc
    if let Some(re) = Regex::new(r#""desc":"([^"]+)""#).ok() {
        if let Some(caps) = re.captures(body) {
            let t = html_decode(caps.get(1).map(|m| m.as_str().trim()).unwrap_or(""));
            if !t.is_empty() {
                return truncate(&t, 20);
            }
        }
    }
    "笔记".to_string()
}

fn extract_author_regex(body: &str) -> String {
    let patterns = [
        r#""nickname":"([^"]+)""#,
        r#""user":"""name"":""([^"]+)"""#,
        r#""author":"""name"":""([^"]+)"""#,
        r#""userInfo".*?"nickname":"([^"]+)""#,
    ];
    for p in &patterns {
        if let Some(re) = Regex::new(p).ok() {
            if let Some(caps) = re.captures(body) {
                if let Some(m) = caps.get(1) {
                    let name = html_decode(m.as_str().trim());
                    if !name.is_empty() {
                        return truncate(&name, 15);
                    }
                }
            }
        }
    }
    "博主".to_string()
}

fn extract_likes_regex(body: &str) -> String {
    let patterns = [
        r#""likedCount":"([^"]+)""#,
        r#""likes":"([^"]+)""#,
        r#""liked_count":"([^"]+)""#,
        r#""interactInfo".*?"likedCount":"([^"]+)""#,
        r#""interactInfo".*?"liked":"([^"]+)""#,
    ];
    for p in &patterns {
        if let Some(re) = Regex::new(p).ok() {
            if let Some(caps) = re.captures(body) {
                if let Some(m) = caps.get(1) {
                    let likes = m.as_str().trim();
                    if !likes.is_empty() {
                        return likes.to_string();
                    }
                }
            }
        }
    }
    "0".to_string()
}

fn download_file(url: &str, path: &std::path::Path, window: &tauri::Window) -> Result<u64, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();
    let resp = agent
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .set("Referer", "https://www.xiaohongshu.com/")
        .call()
        .map_err(|e| e.to_string())?;

    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;
    let mut last_percent: i32 = 10;

    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;

        if total > 0 {
            let percent = ((downloaded as f64 / total as f64) * 90.0) as i32 + 10; // 10-100
            if percent > last_percent {
                last_percent = percent;
                emit_progress(window, percent, "下载中...");
            }
        }
    }

    Ok(downloaded)
}

