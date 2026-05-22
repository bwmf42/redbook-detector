#!/usr/bin/env python3
import sys
import re
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

    vurl = extract_video(html)
    if not vurl:
        print("未找到视频")
        sys.exit(1)
    print(f"视频: {vurl[:70]}...")

    title = clean(extract_title(html))
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
