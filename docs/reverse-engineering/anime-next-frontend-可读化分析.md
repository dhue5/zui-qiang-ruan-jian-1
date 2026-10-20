# anime-next 前端 bundle 可读化分析

> 目标：说明原版前端 bundle 的“可读化”现状——哪些已还原、哪些仍以压缩/PHF 形态存在、能否继续恢复。
> 参考：`outputs/anime-next-frontend/`（已解锁资源）与 `outputs/anime-next-clone/dist/`（运行时 bundle）。

## 1. 已还原为可读资源的部分

原版 `app.exe` 内嵌前端经历过两层编码，REA 已完成解码并归档到 `outputs/anime-next-frontend/`：

- `index.html`：明文还原，含加载逻辑、图标引用与 CSS，全文可读。
- 全部 131 个 JS 文件 + CSS/字体/图片：Brotli 解压后的原始 Vite chunk（`assets_*.js`）。
- PHF（perfect hash）条目表：定位在 `0x214d170`，每条 32 字节
  （`key_ptr u64 + key_len u64 + val_ptr u64 + val_len u64`），键值已全部解出，
  不再存在“字符串找不到”的问题。

也就是说：**资源完整性和运行所需的前端文件本来就齐全**。clone 直接把 `dist/` 原样嵌入
Tauri，UI 行为与原版一致。

## 2. 仍未“源码级可读”的部分

解压后的业务 JS 仍是 **Vite/esbuild 压缩产物（minified）**：

- 变量名已短化（`t/e/s/r/a/f` 等），无原地私有名。
- 无 sourcemap（`*.map` 不存在），无法逐行对照原 TSX/TS 源码。
- 大 bundle 单行化（例如 `assets_index-otr_FE2s.js` 578 KB 基本一行）。

这意味着“还原成可维护源码”而不是“还原成可运行 bundle”这层目标，在没有 sourcemap 的前提下
做不到自动恢复。能做的是：

1. 用格式化器展开成多行（本机 `prettier` 安装失败，网络失败，未执行；可用
   `npx prettier@3.x assets_index-otr_FE2s.js --write` 尝试）。
2. 人工/半自动 AST 重命名：需要用 Babel 遍历并重建语义名，工程量大且会引入偏差。
3. 保持现状：UI 行为 100% 一致，仅维护层面“读源码”受限。

结论：**不影响功能一致性**；属于“逆向完整性”上的已知残余，不阻塞运行。

## 3. 业务模块索引（从 bundle 中确认的调用点）

以下索引来自 `assets_index-otr_FE2s.js` 内的命令/事件调用，可作为手工翻阅时的定位锚点
（均为压缩代码中的可见标识符）：

| 域名 | 锚点字符串 | 说明 |
| --- | --- | --- |
| 对话流式 | `stream_chat_raw` / `chat-raw-chunk` | 前端发起流式对话，收听 chunk/end/error |
| 任务队列 | `submit_generation_task` / `task-update` / `get_pending_tasks` | 生成任务 store |
| 媒体生成 | `generate_media` / `media-progress` | 通用生成 hook |
| 模型配置 | `provider-configs` | localStorage 模型配置 |
| 自定义提示词 | `system-prompts` / `system-prompts-changed` | 提示词库合并 |
| 主题 | `anime-next-theme` | 主题持久化 key |
| 分镜/画布 | `create_canvas` / `update_canvas` | 画布 CRUD |
| 笔记 | `create_note` / `update_note` | 笔记 CRUD |

## 4. 后续可选项

- 若你手上有原版构建的 `vite.config`/`dist/assets/*.map`，把 map 放入即可真正还原源码结构。
- 若要长期维护 clone，建议后续 `vite build` 生成自己的源码，而不是继续 patching 压缩 bundle。
