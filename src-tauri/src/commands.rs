use regex::Regex;
use serde::{Deserialize, Serialize};
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

#[tauri::command]
pub fn download_xhs(url: String, window: tauri::Window) -> XhsResult {
    let desktop = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Desktop"))
        .unwrap_or_else(|_| PathBuf::from("."));

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

    emit_progress(&window, 5, "提取视频地址...");
    let video_url = match extract_video(&resolved) {
        Ok(u) => u,
        Err(e) => {
            emit_progress(&window, 0, "idle");
            return XhsResult {
                success: false,
                message: format!("未找到视频: {}", e),
                path: None,
            };
        }
    };

    let filename = format!("xhs_{}.mp4", chrono::Local::now().format("%m%d_%H%M%S"));
    let filepath = desktop.join(&filename);

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

fn extract_video(page_url: &str) -> Result<String, String> {
    let body = ureq::get(page_url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;

    let re = Regex::new(r#"https://sns-video[^"'\s<>]+(?:mp4|m3u8)[^"'\s<>]*"#)
        .map_err(|e| e.to_string())?;

    re.find(&body)
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "未在页面中找到视频链接".into())
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
