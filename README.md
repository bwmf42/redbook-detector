# redbook-detector

小红书视频下载工具。

## 要求

- macOS
- Python 3
- curl

## 用法

复制小红书链接（短链或长链均可），然后：

```bash
cd redbook-detector
python3 xhs_download.py
```

或直接带链接运行：

```bash
python3 xhs_download.py "https://www.xiaohongshu.com/explore/..."
```

视频自动下载到桌面。

## 限制

- 仅支持公开视频笔记
- 小红书网页结构变更可能导致脚本失效
