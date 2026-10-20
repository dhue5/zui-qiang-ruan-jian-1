# REA 大二进制逆向方法笔记

> 适用：剥离符号的 40-50 MB 级 Windows x64 原生二进制（如 Rust/Tauri 单文件应用）
> 案例：`anime-next` app.exe（45 MB，SHA-256 c0de5de1fa47...）
> 工具：REA 6.5.0 + Ghidra 12.1.4 + JDK 21 + WSL Python（字符串/模式扫描）

---

## 1. 目标形态判定（先做，30 秒）

```powershell
# PE 基本信息 + 版本
Get-Item app.exe | Select VersionInfo

# 技术栈快速判定（PE 导入表 / 特征字符串）
Get-Command dumpbin, objdump | Select Source   # 若有

# 或用 REA（无需 Ghidra）
rea inspect-artifact app.exe --json
rea inspect-managed-artifact app.exe --json      # 是否 .NET
rea inspect-rust-binary app.exe --json           # 是否含 rust-auditable 元数据
```

判定要点：

| 特征 | 结论 |
| ---- | ---- |
| `app.asar` / `node_modules` / `Electron` | Electron JS 应用 |
| `tauri::` / `tauri://localhost` / `wry` | Rust + Tauri |
| `MSVCP140` + UCRT 导入 | MSVC C++ |
| CLI 元数据存在 | .NET 托管 |
| `.dep-v0` 段存在 | 可读 Rust 依赖清单 |

`anime-next` 案例：Tauri 2.x（Rust 编译）+ WebView2 前端，非托管，无 `.dep-v0`（剥离符号）。

---

## 2. 字符串层快速取证（WSL Python，1-3 分钟）

大二进制避免用 PowerShell 全量字符串（内存不足），用 WSL Python：

```python
import re
data = open(path, "rb").read()
# ASCII 可打印串
for m in re.finditer(rb"[\x20-\x7e]{12,}", data):
    print(m.group().decode(errors="replace"))
# 中文 UTF-8 串
pat = re.compile(rb"(?:[\xe4-\xe9][\x80-\xbf][\x80-\xbf]){3,}")
for m in pat.finditer(data):
    print(m.group().decode("utf-8", errors="replace"))
```

重点收集：
- URL / API 端点（`https://`、`wss://`、`/chat/completions`、`Bearer `）
- IPC 命令表（Tauri：命令名 + 参数名常以无分隔符连续串形式存在）
- 结构体字段名（serde：`category_name item_name ...` 相邻出现）
- 错误字符串 + 源码路径（`src\prompts\mod.rs` → 定位模块）
- 内嵌提示词/模板（大段可读文本）

注意：剥离符号后 `search_strings` / `search_procedures` 可能大量为空，字符串层是主要入口。

---

## 3. 文件偏移 ↔ 虚拟地址映射（WSL Python + pefile）

所有后续分析需要 VA，用 pefile 建映射：

```python
import pefile
pe = pefile.PE(path, fast_load=True)
def foff_to_va(foff):
    for s in pe.sections:
        rp = s.PointerToRawData
        if rp <= foff < rp + s.SizeOfRawData:
            return pe.OPTIONAL_HEADER.ImageBase + s.VirtualAddress + (foff - rp)
```

拿到关键字符串 VA 后，用 **RIP 相对引用扫描**定位代码引用点：

```python
# x64 LEA reg,[rip+disp32] = 48/4C 8D xx disp32（xx & 0xC7 == 0x05）
if b in (0x48, 0x4C) and text[i+1] == 0x8D and (text[i+2] & 0xC7) == 0x05:
    disp = struct.unpack_from("<i", text, i+3)[0]
    target = text_va + i + 7 + disp
```

这是 Ghidra xrefs 为空时的替代方案（Rust 常通过寄存器间接加载字符串，不建立数据引用边）。

---

## 4. Ghidra 大二进制加速配置

默认全量分析对 45 MB 二进制 >30 分钟仍超时，用以下组合：

```powershell
$env:REA_GHIDRA_STARTUP_TIMEOUT_MS = '3600000'   # 60 分钟
$env:GHIDRA_HEADLESS_MAXMEM = '5G'               # 堆上限
$env:REA_GHIDRA_DISABLED_ANALYZERS = 'WindowsResourceReference,Decompiler Switch Analysis'
```

说明：
- `REA_GHIDRA_DISABLED_ANALYZERS` 的名字必须是 Ghidra 注册且带 options 的精确名称；
  `Constant Propagation` 这类无 options 的会直接让 pre-script 失败（`Unknown Ghidra analyzer`）。
- 有效示例：`WindowsResourceReference`（官方文档）、`Decompiler Switch Analysis`（实验文档）。
- 禁用会损失该分析器产出的事实，只在定位函数/字符串引用时使用。

分析成功后 REA 会返回函数/段/字符串计数，例如：
`procedures=94683 strings=26040 segments=8`。

---

## 5. REA MCP 单会话驱动（避免每次冷启动）

CLI 每次调用都会重建临时 Ghidra 项目（分析又慢），改用 MCP stdio 保持一个会话：

```mjs
import { spawn } from "node:child_process";
const child = spawn(process.execPath, ["scripts/rea.mjs", "mcp"], {
  env: { ...process.env, REA_GHIDRA_... },
  stdio: ["pipe", "pipe", "pipe"],
});
// JSON-RPC over stdio：initialize → notifications/initialized → tools/call
```

关键工具：

| 工具 | 用途 |
| ---- | ---- |
| `open_binary` | 绑定目标（`provider_id: "ghidra"`） |
| `procedure_info` | 函数入口/长度/基本块（先探测候选入口） |
| `procedure_pseudo_code` | 反编译（传符号名如 `FUN_140466d00`） |
| `search_procedures` | 正则搜 `FUN_<地址前缀>` 定位段内函数 |
| `search_strings` / `search_bytes` | 字符串/字节搜索 |
| `xrefs` / `procedure_references` | 引用分析（剥离符号二进制常为空，勿依赖） |

剥离符号后的定位套路：
1. RIP 扫描拿到引用点（如 `0x14044b8f8`）
2. 对该段 `search_procedures` 正则 `FUN_14044b`
3. 逐个 `procedure_info` 找 body 覆盖引用点的入口
4. 反编译该入口

`procedure_info` 只接受**函数入口地址**；给函数中部地址会返回
`No procedure exists at the requested address`。

---

## 6. 组合逻辑还原流程（三层证据）

```text
第 1 层：字符串
  IPC 命令名 + 参数名（compose_system_prompt categoryId itemId thinkingId outputId）
  结构体字段（base_prompt thinking_prompt output_prompt composed_prompt）
  错误路径（Output format not found / src\prompts\mod.rs）

第 2 层：函数
  RIP 扫描 → 定位字段引用簇 → procedure_info → procedure_pseudo_code
  主逻辑：memcmp 按名称查找 + join(items, "\n\n", 2)
  序列化：按字段顺序写 JSON
  析构：drop 释放各 String

第 3 层：运行时
  DuckDB/SQLite 等应用数据库导出实际生成的数据
  例：messages 表里直接存着以 "\n\n" 开头的完整组合提示词
```

三层证据交叉验证：字符串 → 函数 → 实际数据一致，结论可靠。

---

## 7. 常见坑与对策

| 坑 | 对策 |
| -- | ---- |
| Ghidra 全量分析超时 | 禁用分析器 + 大堆 + 长超时；或 MCP 单会话 |
| `search_strings` 找不到长提示词 | 长文本常是 undefined data，改用字节搜索或 raw 扫描 |
| `search_procedures` 空 | 剥离符号 → 用 `FUN_<前缀>` 正则 |
| `xrefs` 全空 | Rust 寄存器间接加载，RIP 相对引用扫描替代 |
| 反编译传错地址 | 先 `procedure_info` 拿入口，再传符号名 |
| PowerShell 字符串处理 OOM | 用 WSL Python + 正则流式处理 |
| MCP 驱动脚本转义地狱 | 脚本文件方式写 Python/JS，避免多层引号 |

### 附加：前端资源与跳转表边界

- **前端 bundle 提取**：Tauri exe 内 Tauri JS API 封装常以连续可打印段存在
  （`__TAURI_INTERNALS__` / `__TAURI__` / `invoke(`），从标记点向后/前扫描
  可打印 run 边界即可还原。若 `createRoot`/`React` 计数为 0，说明 React 业务
  代码未以纯文本嵌入（可能走 `localhost:<port>` 或额外资源）。
- **跳转表解析**：`LEA RCX,[base]; MOVSXD RAX,[RCX+idx*4]; ADD RAX,RCX;
  JMP RAX` 是 Rust switch 标准模式，分支目标 = `base + idx*4 + sign(dword)`
  （不是绝对地址）。分支常在函数中部，禁用部分 Ghidra 分析器后可能被标为
  `undecodable`；此时 `procedure_assembly` 会报 `No procedure exists`，
  属分析边界而非地址错误。
- **Rust 结构体字段还原**：`drop` 函数（逐个释放 String）与序列化函数
  （按序写 JSON key）字段顺序一致，可互相印证结构体布局。

### Tauri 嵌入式前端资源破解（PHF + Brotli）

**关键判定**：Tauri 2.x `EmbeddedAssets` 在 `compression` feature 下用
**Brotli** 压缩每个资源（tauri-utils `assets.rs` 源码确认
`brotli::BrotliDecompress`）。不要被"路径后高熵字节"误导去猜
zstd/lz4/加密。

**破解步骤**：

```text
1. 定位资源路径字符串（/assets/index-xxx.js）→ 它们就是 PHF 表 key
2. 找到 entries 表：Rust phf 生成的布局为
   [key_ptr u64][key_len u64][val_ptr u64][val_len u64]，每 32 字节
   一条；通过搜索 (key_va, key_len) 的 u64 对定位表基址
3. 按 val_ptr/val_len 切出压缩字节
4. brotli.decompress() 逐个还原
```

**经验**：
- 路径字符串前 32 字节是 PHF disps/hash 数组（`02000000 8c000000...`），
  不是数据头
- key 与数据表分离：路径集中存，数据由 entry 的 val_ptr 指向
- 压缩资源开头 `1b...` / `1b cf ac cc` 是 Brotli 帧头，不是常见
  zstd（`28 b5 2f fd`）/ gzip（`1f 8b`）魔数
- 还原后前端业务代码（React/Vite）立即可搜：`compose_system_prompt`、
  `createRoot` 等
- 本案例：197 个资源（7.8 MB，131 个 JS chunk）全部还原，0 失败

---

## 8. 可复用脚本清单（work/）

- `peinfo.py`：PE 段/导入表
- `strings.py` / `cn_prompts.py`：ASCII/中文串提取
- `va_map.py`：文件偏移→VA
- `rip_scan*.py`：RIP 相对引用扫描（按目标 VA 区间）
- `rea_mcp_drive.mjs`：REA MCP 单会话框架
- `rea_schemas.mjs`：工具 schema dump
- `rea_*.mjs`：xrefs / 反编译 / 定位驱动脚本

---

## 9. 结论模板

最终报告应包含：目标识别（名称/版本/SHA/大小）、技术栈结论、
关键字符串证据、函数级反编译（函数地址+伪代码摘要）、运行时实证、
局限说明（剥离符号、分析器禁用、未覆盖范围）。

参考交付：
- [anime-next-内置提示词.md](/C:/Users/Y028/Documents/Codex/2026-10-16/github-rea/outputs/anime-next-内置提示词.md)
- [anime-next-reverse-decompile.txt](/C:/Users/Y028/Documents/Codex/2026-10-16/github-rea/outputs/anime-next-reverse-decompile.txt)
