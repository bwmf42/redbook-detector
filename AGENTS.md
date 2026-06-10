# redbook-detector Agent Guide

本文件只记录 `redbook-detector` 的项目专属规则。通用称呼、中文回复、非琐碎任务确认、手术式修改、git 安全和敏感信息规则见父级 `/Users/vigo/Desktop/code/AGENTS.md`。

## 项目定位

- 这是小红书公开视频下载工具。
- 当前有两个入口：`xhs_download.py` Python CLI，以及 Tauri 桌面应用。
- 目标平台是 macOS；Python CLI 依赖 `python3`、`curl` 和系统剪贴板 `pbpaste`。
- 小红书网页结构经常变化，解析失败不一定是本地代码回归，先用真实链接验证页面结构。

## 关键目录和文件

- `xhs_download.py`：Python 命令行下载器；不传参数时读取剪贴板，默认下载到桌面。
- `src/`：Tauri 前端，原生 HTML/CSS/JS。
- `src-tauri/src/commands.rs`：Tauri 下载命令、链接解析、视频信息提取、进度事件、Finder 打开。
- `src-tauri/src/main.rs`、`src-tauri/src/lib.rs`：Tauri 应用入口和命令导出。
- `src-tauri/Cargo.toml`：Rust/Tauri 依赖。
- `.dmg`、`.zip`、`.app`：构建产物，不要手工编辑。

## 修改红线

- 解析逻辑优先从 `window.__INITIAL_STATE__` 提取，再使用正则回退；不要只依赖脆弱正则。
- 下载请求需要保留合适的 `User-Agent` 和 `Referer`。
- 文件名必须继续做非法字符清理，避免写入失败或路径穿越。
- 不要添加登录态抓取、Cookie 持久化、批量爬取或绕过访问限制的能力，除非用户明确提出并确认合规风险。
- 前端保持小工具形态，优先清楚的输入、进度、错误和保存路径，不做营销页。

## 验证命令

Python CLI 语法检查：

```bash
python3 -m py_compile xhs_download.py
```

Python CLI 真实链接验证：

```bash
python3 xhs_download.py "https://www.xiaohongshu.com/explore/..."
```

Tauri/Rust 检查：

```bash
cd src-tauri
cargo check
```

涉及打包、权限、对话框或 Finder 打开时，再运行 Tauri 构建或手动启动应用验证。
