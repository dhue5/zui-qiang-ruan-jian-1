# 最强软件

最强软件是一个基于 Tauri 2 + Rust + React 的 AI 创作工作台，包含聊天、提示词组合、笔记、画布、媒体库和生成任务队列。

## 运行

直接运行发布包中的 `zui_qiang_ruan_jian.exe`，或安装：

- `src-tauri/target/release/bundle/nsis/最强软件_1.0.2_x64-setup.exe`

首次使用时，在设置中配置模型服务商和 API Key。模型配置保存在用户本地，不写入仓库和安装包。

## 构建

环境要求：Node.js、Rust stable-msvc、Visual Studio Build Tools（MSVC + Windows SDK）。

```powershell
npm install
cargo test --manifest-path src-tauri/Cargo.toml --release -- --test-threads=1
npx tauri build
```

发布构建（自动递增 patch 版本号、测试、构建、品牌核对）：

```powershell
pwsh scripts/build-release.ps1
```

## 本地升级

将发布脚本生成的 `scripts/update-manifest.json` 复制到：

```text
%APPDATA%\com.zuiqiangruanjian.desktop\update-manifest.json
```

清单可以使用 `file:///` 本地安装包路径。`settings.json` 中将 `auto_install_updates` 设为 `true` 可启用静默覆盖安装。

## 数据迁移

`scripts/migrate-legacy.js` 支持把原版 DuckDB 导出的 JSON 导入新 SQLite 数据库。迁移脚本不会修改原版数据库。

## 说明

本仓库不包含 `node_modules/`、Rust `target/`、用户数据库、媒体文件或 API Key。请勿把真实凭据提交到 Git。
