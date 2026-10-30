# redbook-detector 项目交接

更新时间：2026-09-30（Asia/Shanghai）。本次为文档整理，源码基线取自修改文档前的工作树。

- 分支 / HEAD：`main` / `4c3623f`。
- 远端状态只读取本机缓存的跟踪引用，未执行 fetch；不据此宣称 GitHub 或 NAS 已同步。
- 已核对：本地说明、相关入口源码与 Git 状态；线上服务、已安装 App 和真实账号流程未复验。

## 当前交付与已提交工作

macOS 小红书公开视频下载工具，同时有 [Python CLI](xhs_download.py) 与 Tauri 桌面入口 [commands.rs](src-tauri/src/commands.rs)。先读 [AGENTS.md](AGENTS.md) 和 [README.md](README.md)。

- 2026-08-03 `4c3623f`：同步本地变更并对齐私有远端。
- 2026-05-22 `3d40827`：优先解析 `__INITIAL_STATE__`，加入 HTML 双重解码。
- 同日 `bdae73e`：自定义保存目录与「博主_标题_赞数」命名。
- 本地没有更晚的已提交功能记录。

## 未提交工作（接手重点）

`src-tauri/src/commands.rs` 有 53 行新增、1 行删除的已有修改：增加 `extract_xhs_url`，先从分享文案中提取链接，再解析跳转；无链接时返回明确错误。涉及 `xhslink.cn`、长链接、补协议和纯 URL 回退。

这部分尚未提交，本次没有改动或替它宣称验收通过。后续优先核对短链域名覆盖、标点尾缀、整段分享文本以及域名判断边界，再决定是否提交；不要用覆盖工作树的方式同步远端。

## 验证与下一步

本次只检查 Git 差异与源码，没有真实下载、编译或安装。后续验证可从 `python3 -m py_compile xhs_download.py`、`cd src-tauri && cargo check` 开始，再用获准的公开视频链接测试。

保留请求 UA/Referer 与文件名清理。站点结构漂移可能导致失败；没有真实链接复验时不能写“下载已恢复”。不要新增登录态采集或读取用户 Cookie 来补足文档证据。

## 本次收尾范围

- `verified-current`：本地 Git/文件状态及上文点名的源码证据（非 Git 目录仅文件快照）。
- `changed-and-verified`：交接文档及接手入口，已做本地链接与差异检查。
- `pending`：上文未执行或受阻的产品/运行验证。
- `out-of-scope`：部署、安装、远端同步、数据清理及真实账号操作；长期记忆只读。

## 2026-10-31 审计修正（未提交）

- `extract_xhs_url` 改为单个 `OnceLock` 正则：只吃 URL 合法 ASCII 字符，遇中文/全角标点/空白即止，去结尾 `.,;:!?)'`；支持 `xhslink.com/.cn`、`(www.)xiaohongshu.com`，有无协议均可（无协议补 https，有协议原样保留），取第一个链接，域名前有边界检查。
- 新增 7 个 Rust 单元测试；`src/app.js` 剪贴板检测改为匹配 `xhslink|xiaohongshu.com`。
- 验证：`cargo test` 7 通过、`cargo check` 通过、`py_compile` 通过；真实链接下载与 App 打包未验证，待 Vigo 验收。
