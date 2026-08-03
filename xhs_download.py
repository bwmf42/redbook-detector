#!/usr/bin/env python3
import sys
import re
import json
import subprocess
from pathlib import Path

UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"
REFERER = "https://www.xiaohongshu.com/"


def clipboard():
    r = subprocess.run(["pbpaste"], capture_output=True, text=True, timeout=5)
    return r.stdout.strip()


def fetch(url):
    req = __import__("urllib.request").request.Request(url, headers={"User-Agent": UA})
    with __import__("urllib.request").request.urlopen(req, timeout=30) as resp:
        return resp.geturl(), resp.read().decode("utf-8", errors="ignore")


def pick_best_stream(stream):
    """从 stream 对象中选最高清的视频 URL"""
    candidates = []  # (score, url)
    for codec in ("h265", "h264", "av1"):
        entries = stream.get(codec, [])
        if not isinstance(entries, list):
            continue
        for entry in entries:
            url = entry.get("masterUrl", "")
            if not url:
                continue
            w = entry.get("width", 0) or 0
            h = entry.get("height", 0) or 0
            bitrate = entry.get("videoBitrate", 0) or entry.get("bitrate", 0) or 0
            score = w * h * 1000 + bitrate if w and h else bitrate
            candidates.append((score, url))
    if not candidates:
        return None
    candidates.sort(key=lambda x: x[0], reverse=True)
    return candidates[0][1]


def extract_from_initial_state(html):
    """从 __INITIAL_STATE__ 解析视频信息，返回 (video_url, title) 或 None"""
    m = re.search(r'window\.__INITIAL_STATE__\s*=\s*(\{.+?})\s*</script>', html, re.DOTALL)
    if not m:
        return None
    raw = m.group(1)
    # 处理非法 JSON 值
    raw = re.sub(r':undefined', ':null', raw)
    raw = re.sub(r':NaN', ':null', raw)
    raw = re.sub(r':Infinity', ':null', raw)
    raw = re.sub(r':-Infinity', ':null', raw)
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        return None

    # 定位 note.noteDetailMap.{noteId}.note
    note_map = data.get("note", {}).get("noteDetailMap", {})
    if not note_map:
        return None
    detail = next(iter(note_map.values()), {})
    note_obj = detail.get("note", {})

    title = note_obj.get("title", "").strip()
    if not title:
        title = note_obj.get("desc", "").strip()

    video = note_obj.get("video", {})
    stream = video.get("media", {}).get("stream", {})
    video_url = pick_best_stream(stream)
    if not video_url:
        # 兜底: 取 h264[0]
        h264 = stream.get("h264", [])
        if h264:
            video_url = h264[0].get("masterUrl", "")
    if not video_url:
        return None

    return video_url, title or "xiaohongshu_video"


def extract_video(html):
    m = re.findall(
        r'https?://sns-video-[^"\s<>]+\.mp4\?[^"\s<>]*sign=[^"\s<>]*', html
    )
    return m[0] if m else None


def extract_title(html):
    for p in [
        r'<meta[^>]*property="og:title"[^>]*content="([^"]*)"',
        r'<title[^>]*>([^<]*)</title>',
    ]:
        m = re.search(p, html)
        if m:
            return m.group(1).strip()
    return "xiaohongshu_video"


def download(url, path):
    r = subprocess.run(
        ["curl", "-sL", "-A", UA, "-H", f"Referer: {REFERER}", "-o", str(path), url],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if r.returncode != 0:
        raise RuntimeError(r.stderr)
    return path.stat().st_size


def clean(name):
    name = re.sub(r'[\\/:*?"<>|]', "_", name).strip().strip(".")
    return name or "video"


def main():
    if len(sys.argv) >= 2:
        url = sys.argv[1].strip()
    else:
        url = clipboard()
        if not url:
            print("用法: python3 xhs_download.py [链接]")
            print("  不传参数时自动读取剪贴板")
            sys.exit(1)
        print(f"剪贴板: {url[:70]}...")

    if not url.startswith(("http://", "https://")):
        print(f"无效链接: {url[:70]}")
        sys.exit(1)

    print(f"解析: {url[:70]}...")
    real, html = fetch(url)

    # 优先从 __INITIAL_STATE__ 提取最高清流
    result = extract_from_initial_state(html)
    if result:
        vurl, title = result
        print(f"[高清] 视频: {vurl[:70]}...")
    else:
        # 回退: 正则匹配
        vurl = extract_video(html)
        title = extract_title(html)
        if not vurl:
            print("未找到视频")
            sys.exit(1)
        print(f"视频: {vurl[:70]}...")

    title = clean(title)
    desk = Path.home() / "Desktop"
    out = desk / f"{title}.mp4"
    i = 1
    while out.exists():
        out = desk / f"{title}_{i}.mp4"
        i += 1

    print(f"下载 -> {out.name}")
    size = download(vurl, out)
    print(f"完成 {size / 1024 / 1024:.1f} MB")


if __name__ == "__main__":
    main()
