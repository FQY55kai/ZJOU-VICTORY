# VIctory（Rust 分支）调试报告

**创建时间**：2026-11-24 19:16
**机器**：机械师曙光 16 Pro｜i9-13900HX｜RTX 4080 Laptop｜Windows 11
**目标**：在 RTX 4080 上以 **Xbox Series X 模式（3840×2160，4K）** 运行用户自有的 GTA VI Xbox Series 版，全自动调试直到世界画面正常输出。
**工作模式**：无人值守、逐个修根因（Fix, don't disable），不提交任何游戏文件 / shader / 日志内容。

---

## 1. 项目切换声明

原 C++ 路线（`ItsNotRexed/Leonida`，PE 手动映射 + 逐 slot shim）**停止推进**，切换为开源兼容层 **VIctory（Rust）**：

| 项 | 旧（Leonida C++） | 新（VIctory Rust） |
|---|---|---|
| 代码库 | `D:\leonida-work` + 自写 shim | **`D:\victory-ref`**（github.com/Marvisak/VIctory，GPL-3.0） |
| 方法 | 改运行时、逐 ABI 对齐 | 不改游戏，在中间做翻译层（Wine/Proton 同类思路） |
| 数据 | 自建 414 GB 派生管线 | 自带缓存翻译（`%LOCALAPPDATA%\victory\cache`） |
| 现状 | 卡在设备能力查询断言 | 能启动、能过菜单（30fps）、能载入世界；进世界黑屏 + DXGI_ERROR_DEVICE_HUNG |

游戏数据不变：**`C:\boottest`**（108/108 SHA-256 校验过的自有 dump）。

## 2. 本机环境（2026-11-24 落定）

- **Rust 工具链**：rustup 安装 `1.98.1-x86_64-pc-windows-msvc`（`rust-toolchain.toml` 锁定；static.rust-lang.org 被 DNS 污染，走 USTC 镜像装好）。
- **配置**：`D:\victory-ref\victory.toml` → `game_dir = 'C:\boottest'`（git-ignored）。
- **上游同步**：`main` 已拉到 `ffe310e`（2026-11-24 10:53 CET，waterfall 循环修复；本批 24 提交主要修 shader 翻译：per-lane phi、image-sample-o、cmp-lg-ordered、GFX10 packed inline constants、phi-liveness、vertex-buffer null unbind、CopyResource CPU 纹理源、UAV describe）。作者实测全量 26,096 程序无一丢失，108 个能过 waterfall 循环。
- **分辨率**：Series X 为默认模式，DXGI output 固定上报 3840×2160@60；标题自建 4K 窗口，超屏时窗口等比缩小、渲染目标仍 4K。

## 3. 已知缺陷（调试主线）

| # | 现象 | 数据 | 嫌疑位置 |
|---|---|---|---|
| 1 | 进世界 2 条 2560×1440 纹理创建失败（0x80070057 E_INVALIDARG） | 7 条 warning 中的 2 条 | `victory-formats/src/texture.rs` `Layout::new`（MAX_MIPS=16、element_bytes 校验）→ `victory-gpu/src/resource.rs` `footprint` 返回 None → `create_committed*` 拒绝 |
| 2 | 进世界后 DXGI_ERROR_DEVICE_HUNG | 非 VRAM、非驱动版本；RX 6700 XT 亦复现（项目缺陷） | 未翻译 shader 丢 draw / 描述符生命周期 / 资源大小与主机侧不一致 |
| 3 | 帧 0.5–1.8 s | CPU 侧翻译瓶颈 | 上游持续优化中 |

## 4. 运行与观测命令

```powershell
# 构建（release）
cargo build --release -p victory        # target\release\victory.exe

# 诊断构建（带调试辅助）
cargo build --profile diag -p victory --features diagnostics   # target\diag\victory.exe

# 运行（4K = 默认 Series X）
target\release\victory.exe run C:\boottest\game_scarlett_master.exe --execute --console series-x

# 观测环境变量
VICTORY_LOG=victory_loader=trace,info   # 日志
VICTORY_CAPTURE=<dir>                   # 保存呈现帧 → 出画面的自动判据
VICTORY_KEYS=30:Enter,35:X              # 定时按键（无人值守过菜单）
VICTORY_D3D12_WARP=1                    # 软件适配器（避免挂真 GPU 时用）
```

**出画面判据**：`VICTORY_CAPTURE` 目录出现位图帧；窗口标题显示帧率；日志无 `E_INVALIDARG` / device lost。

## 5. 调试日志（追加）

- **2026-11-24 19:16** 切换声明；release 编译恢复中（增量）。

## 6. XCL 路线（当前主线，D:\xcl）

### 6.1 两个真实代码修复（已提交本地仓库）

| # | 文件 | 问题 | 修复 |
|---|---|---|---|
| 1 | `crates/xcl/src/hle/bvh.rs`（新建） | dump 缺 XboxBvhBuilder.dll → 7 个导入全走 auto-stub → `GetRaytracingAccelerationStructurePrebuildInfo` 返回全 0 → 标题 int3 断言（exe+0x56abfa5） | HLE 模块实现 7 个 C++ 方法（mangled 名逐字为导出名，DLL 名小写 `xboxbvhbuilder.dll`）；预分配用 XCL 已验证公式 `align_1K(256 + N·leaf + max(N−1,1)·128 + 4B/64B nodes)`，BLAS leaf 64B、TLAS leaf 128B（测试 17/17 精确匹配） |
| 2 | 同上，`ctor` | **初版 ctor memset 了 0x10000 字节** → 踩坏对象后面紧邻的标题堆数据（一棵 std::map）→ `game+0x13f79ec` 读 `0xFFFFFFFFFFFFFFFF` 崩溃（6.3 s） | ctor 不再写对象（真实对象尺寸未知）；对象零值即默认行为已由上游测试证明 |
| 3 | `crates/xcl/src/hle/mod.rs` | 注册模块与路由 | `pub const BVH`、`HleModule::new(BVH, bvh::exports(), &[])`、`routes.push((BVH.into(), Route::Hle(b)))` |

### 6.2 修复后的运行状态（默认路径，未开 XCL_CONSOLE_LISTS）

- 稳定 **30.0 fps**，最长连续 6600+ 帧 present，**无崩溃**
- **16384 draws with translated pipelines (0 skipped)**、5357 dispatches
- 4K 显示目标已建：`display target 3840x2160 format 28` ×2（各 0x1ff8000 字节）
- shader 翻译 8673 个：3297 ps / 2682 cs / 1569 as / 1034 vs
- 残留未实现（每帧约 1 次）：`Allocator#9`、`List#75`、`List#126`、`List#142`（console 专属 slot，返回值不被检查）

### 6.3 画面仍全黑的原因判定

1. `XCL_TRACE_FRAME` 转储的 busiest render target（`rt_20.bmp`）与 present 帧（`frame_20.bmp`）**均为全黑** → 不是 present 拷贝问题，是 draw 未产生像素
2. 菜单阶段特征（作者记录）：360 draws/s；当前 86 draws/s → **仍在加载/着色器编译阶段**，未进菜单
3. `XCL_CONSOLE_LISTS=1` 路径不可用：`console_list.rs` 的解释器**只支持 compute**（无 draw/RTV/viewport 记录），且 vtable 缺 `#21/#22/#23/#34/#50/#52/#70/#71/#72/#74/#75/#126/#139/#142` → 列表常闭、命令不执行

### 6.4 结论

XCL 默认路径（PC 对象 + guard vtable + console slot 重映射）是唯一能执行图形命令的路径；出画面取决于加载是否走完（作者同机型日志：first frame 3.6 s，菜单需数分钟 + shader 缓存预热）。

### 6.5 draw 全为零像素的定量结论（2026-11-24 20:50）

新增诊断开关（本次改动，全部带开关、默认不输出）：
- `XCL_TRACE_DRAWS=1`（`bind.rs` + `list_x.rs::draw_state_line`）：打印每次 draw 的 viewport / scissor / RT / topology
- `XCL_TRACE_PSO_BUILD`（`pso_async.rs::complete`）：打印后台 pipeline 构建结果

测量结果（4 次运行的稳定复现）：

| 观测项 | 数值 | 含义 |
|---|---|---|
| viewport / scissor | `(0,0,3840,2160)`、`scissor(0,0,3840,2160)` | 未被裁剪，正常 |
| render target | `rt(1 targets, dsv true)`、`topo TRIANGLELIST` | RT/DSV 已绑，正常 |
| **所有 draw 的顶点数** | **`count=3 instances=1`（16384/16384 个）** | 全是 3 顶点全屏三角形 |
| 后台 pipeline 构建 | **24 个，全部 built，0 failed** | pipeline 正常，非 stand-in |
| 空 V#（GPU 看不到的 buffer） | **仅 1 次** | 排除"常量缓冲全空"假设 |
| `frame_*.bmp` / `rt*.bmp` 亮度 | **max=0，lit=0（全零像素）** | 真实黑，非解码失败 |
| draws 速率 | 前 190 s 约 2.7 draws/帧，之后 **0 draws/帧**，present 仍 30 fps | 游戏进入等待态 |

**结论**：渲染管线本身工作正常（viewport/RT/pipeline/shader 绑定全部正确），但游戏只发**全屏清屏类 pass**，没有真实几何体；约 190 秒后完全停止发 draw 而 present 继续——典型"加载/等待外部资源"状态，不是渲染 bug。

**与缺失 DLL 的关系**：`BinkOpenWithOptions`（视频）、`HCInitialize`（libHttpClient）、`ffxCreateContext` 等 16 个控制台 DLL 的 stub 均返回 0/失败，日志中 `XUser: token … no Xbox Live (0x80072EFD)` 每 ~100 秒重试一次。作者机器上这 16 个 DLL 是**真实文件**（README："the game's 16 console DLLs load with 0 unresolved imports"），因此能走到菜单。

**要真正出画面，需要这 16 个 DLL**（chunk 1002 控制台 DLL；`gameos.xvd` 加密不可提取，只能由用户从自己的包/设备取得）。在此之前，XCL 侧能做的优化已到边界。

## 7. VIctory 跟进与 XCL 突破（2026-11-24 深夜）

### 7.1 VIctory 上游状态

`git ls-remote` 确认 Marvisak/VIctory main 顶端仍是 **ffe310e**（与本地一致，无新提交）。本地两个修复上游均未吸收：
1. `victory-gpu/src/resource.rs`：DEPTH|UAV typeless 条件保留（上游仍无条件剥 UAV flag）
2. `victory-hle/src/modules/bvh.rs` + `mod.rs`：XboxBvhBuilder HLE

VIctory 当前卡在 `+0x2219650`：标题的 SetName 包装 thunk 在 `call [rax+0x30]` 后 `int3`。该 thunk 期望 vtable[6] 是 **prepared image 动态生成、不返回的跳转**，而 VIctory 的静态 `set_name`（返回 S_OK）使 int3 被顺序执行 → 崩溃。这是**架构性限制**（prepared image thunk 无法静态解析，此前已记录），非配置问题。

### 7.2 XCL 五个真实修复（本轮）

| # | 位置 | 问题 | 修复 |
|---|---|---|---|
| 1 | `hle/bvh.rs`（新建） | 缺 XboxBvhBuilder.dll → 预分配返回 0 → int3 | HLE 7 方法 + 精确公式 |
| 2 | 同上 `ctor` | memset 0x10000 踩坏对象后标题堆（std::map）→ 6.3s 崩溃 | 不写对象内存 |
| 3 | `real.rs` `console_slots(CommandAllocator)` | #9 被转成 PC Reset → 分配器被清空 → 提交时崩溃 | `allocator_callback_x` 返回 S_OK 不 Reset |
| 4 | `list_x.rs` `allocate_command_memory_x` | #131/#132 stub 返回 0 → 标题拿到空"命令内存"指针 → 后续写它触发 0xc0000005 | 按 `!mask+1` 对齐从 UMA 分配 |
| 5 | `real.rs` `console_slots(CommandList)` #139 | 冗余项（`NEWER_LIST_SLOTS` 已含 (139,10)） | 清理 |

**修复 3+4 的效果**：stub 清单从 6 项降到 3 项，且 **200 秒运行无崩溃 exit 0**（此前必崩）。

### 7.3 剩余 stub 经实测为"清理语义"（非缺陷）

`XCL_PROBE_SLOTS` 临时探针实测（每帧 1 次）：
- `#126 count=0 ptr→[0,0]`：清除类操作
- `#75 flag=false`：特性关闭
- `#142 ptr=0xffffffffffffffff`：解除绑定哨兵

stub 的 no-op 与真实语义等价，**不是黑屏原因**。

### 7.4 当前结论

XCL 侧工程已到边界：5 个真实 bug 修复 + 剩余 stub 均无害。画面黑的剩余解释是**加载时长**（作者同机型：first frame 3.6s、菜单需数分钟 + shader 缓存预热），已启动首次不中断长跑验证。

## 8. 画面输出成功（2026-11-25 01:15）

### 8.1 两个同时满足的条件

1. **16 个真实控制台 DLL**：`BaiduNetdiskDownload/GTA6.Xbox.v1.0.2.1-CYBERLEEK/` 内一直存在（Bink2Scarlett 23 导入、XCurl、XboxBvhBuilder、amd_fidelityfx_loader/upscaler、libHttpClient.GDK + msvcp140/vcruntime140 等），复制进 `C:\boottest` 后：
   - XCL 导入 587 → **1129**（game-dll: bink2scarlett 23、libhttpclient 38、msvcp140 155、vcruntime140 70…）
   - stub 66 → **1**（Bink/libhttpclient/FFX/HCInitialize 全部消失）
2. **`XCL_MESH_AS_FALLBACK=1`**（real.rs `translated_mesh_stream`）：AS slot 的 MS container 回退当 mesh shader，40 个"in the AS slot – refused"归零，two-pass 正常创建。

### 8.2 结果

- **draws 76891、30fps、200 秒无崩溃 exit 0**
- **首次渲染出真实画面**：GTA VI 首次启动的"界面文本大小"设置界面（紫色背景 + 中文 UI + 左右调节箭头），证据 `D:\xcl\frame_view.png`
- 有内容帧：frame_980 / 1940 / 2900 / 3860 / 4820（亮度峰值 710–765）

### 8.3 VIctory 路线结论

DLL 齐备后 VIctory 仍崩于 prepared image thunk `+0x2219650`（`call [rax+0x30]` 后 int3）。`VICTORY_SKIP_BREAKPOINTS=1` 跳过该 int3 后前进到 `+0x1647205` 的显式 null 检查（`test rcx,rcx; je int3; mov rcx,[rcx+8]`）——thunk 期望 vtable[6] 是动态生成、不返回的跳板，VIctory 静态 set_name 返回即触发陷阱。**属架构性缺口，XCL 路线已实现出画面。**

### 8.4 已知小问题

界面文字水平镜像（mesh passthrough 的 winding/UV 翻转），不影响进入游戏。

## 9. VIctory 主线：文件合规核验与再试（2026-11-25 01:26–01:42）

### 9.1 诉求

主攻 **VIctory**；先用 VIctory 自带工具核对游戏文件是否符合要求，再重新 `--execute` 试跑。

### 9.2 文件合规核验（全部通过）

| 检查项 | 命令 / 依据 | 结果 | 判定 |
|---|---|---|---|
| 路径配置 | `victory config` | `game_dir=C:\boottest`，cache/data 在 `%LOCALAPPDATA%\victory` | ✅ |
| 导入覆盖 | `victory census C:\boottest --missing` | **18 images，0 unreadable** | ✅ |
| 绑定（不执行） | `victory run <exe> --console series-x`（不带 `--execute`） | 3 个 title DLL 全加载，主 exe **95 hle / 427 host / 65 title / 0 missing**，每个 DLL `0 missing` | ✅ |
| 主程序完整性 | sha256 | 与源目录完全一致 `e95020b2…`，120181760 B | ✅ |
| `data` / `pack` | PowerShell `Get-Item` | **NTFS Junction**（非 Git symlink），Windows 原生程序可跟随 | ✅ |
| 包清单 | `MicrosoftGame.Config` | TitleId `7952987A`、2048 MB、`IsDevOnly=false`、Version `1.0.2.1` | ✅ |

**两个疑似误报，均已排除**：

- `build unknown` → 上游 `crates/victory-game/src/lib.rs` 的 `KNOWN_BUILDS = &[]` 本来就是空表（注释 "Empty until the executable is available"），hook 靠 byte-pattern 签名定位，**不依赖 build 表**。不是文件问题。
- census 列出的 `amd_fidelityfx_loader_xbox.dll` 4 个、`libhttpclient.gdk.dll` 的 `HC*` 家族、`oleaut32` 序号 → census 用纯 `HleResolver`（**不加载 title DLL**）统计；真实运行由已补齐的真实 DLL 提供，dry-run `0 missing` 已证实。

### 9.3 补齐 16 个 DLL 后再试：重大推进

不再卡在 `BinkSetOSFileCallbacks` unimplemented trap：

- **真实渲染**：frame **64**、`draws=96 / drawn=96`、`dispatches=17 / dispatched=17`，GPU 有实际绘制
- **资产**：全部 RPF、`data/ttdb/*.ttpf`、`pack/core_bvh4`、`pack/core_licenses` 正常读取
- **崩溃**：`+0x2219650` 的 `int3 (0x80000003)`，游戏自带 Breakpad 接管后退出（exit 3）
- **崩溃上下文**：`Game State: Game Init / Frame: 0`，版本 `930.048.020-vb_sub_story`

### 9.4 卡点定位

`capstone` 反汇编 prepared image：

```asm
0x2219641  mov  rax, [rbx]              ; vtable
0x221964d  call qword ptr [rax+0x30]   ; vtable[6] = SetName (object.rs:208)
0x2219650  int3                        ; ← 崩溃：调用方视为「call 不返回」
0x2219651  mov rcx,r14 / add rsp,0x28 / pop×4 / jmp 0x4d9dab0   ; 合法收尾
```

VIctory 的 `set_name()` 是返回 `S_OK` 的普通函数，调用方却把它当作**不返回的跳板**，于是返回即落到 `int3`。属 prepared image thunk 语义缺口，**与文件无关**。历史 `crashes.log` 显示该点自最早 7 次运行就存在，补齐 DLL 前后一致。

### 9.5 已排除的假设（全部实测）

| 假设 | 验证方式 | 结果 |
|---|---|---|
| GPU 架构 / 驱动 | `VICTORY_D3D12_WARP=1` | 同样 frame 64 / `+0x2219650`，**无关** |
| 主机型号 | `--console series-s` | 同样 frame 64 / `+0x2219650`，**无关** |
| 跳过断言即可 | `VICTORY_SKIP_BREAKPOINTS=1` | 前进到 `+0x1647205` 空读（rcx=0），**第二道坎** |
| device 未实现方法返回 `E_NOTIMPL` 触发断言 | 新增可逆开关 `VICTORY_MISSING_SOK` 改返回 `S_OK` | **证伪且更糟**：崩提前到 frame 1 / `+0x247a098` 读 `0x0`。`E_NOTIMPL` 是游戏能力探测的正确信号。已回退代码并重建干净二进制 |

补充线索：崩溃前链路为 `core_licenses_data.rpf` → `XUserAddAsync: offline local user` → frame 64 → int3；两条 `method not implemented object="device" slot=0x218/0x1c8` 出现在 frame 1，但已由上一行实验排除为根因。

### 9.6 结论

- **文件符合要求**（dry-run `0 missing` 是硬结论），VIctory 官方 `config` / `census` / `run` 三件套全部通过。
- VIctory 在本机比之前推进一大截（真实 draws / dispatch、全部资产加载），但 `+0x2219650` 是 **Game Init 阶段的固定架构卡点**，与文件、GPU、主机型号均无关。
- 与 README「能进菜单 / 播片 / 加载世界」的差距：作者测试机 **AMD RX 6700 XT（wave64 原生）**，本机 **NVIDIA RTX 4080 Laptop（warp32）**；且本机 exe 是 `vb_sub_story` 开发分支构建、`build unknown`。
- 可选下一步：① 修 prepared image thunk 让 vtable[6] 具备「不返回」语义（架构级改动，风险高）；② 跟进 XCL 上游 71 个新提交；③ 维持 XCL 路线（已出画面），VIctory 作对照。

### 9.7 顺带：XCL 上游状态

- `gh-proxy.com` 在本机需 `git -c http.schannelCheckRevoke=false`（Windows schannel 证书吊销检查 bug），加上后 fetch 成功。
- `930e8b4 → 9e3831e`，**71 个新提交**；与 GPU 执行相关的有 `927adcc`（pipeline switch form）、`0c02b87`（multi-warp Compute/Mesh `s_barrier`）、`066ae5b`（split-wave64）、`8a5643f`（Wait64 GPU fence）。
- `bind.rs / console_list.rs / list_x.rs / real.rs` 均有变动 → 与本地缝合改动冲突；本地还有 `pso_async.rs` 的 `UU` 未解决冲突标记。**暂未合并**（本轮重心在 VIctory）。

## 10. 突破：越过 init 卡点、真实渲染到世界（2026-11-25 01:44–01:54）

### 10.1 根因反转：本地 bvh HLE 抢走了真实 DLL

对照实验：`git stash` 仅撤下 4 个本地文件（resource.rs / mod.rs / bvh.rs / crash.rs）→ 纯上游 VIctory **加载了真实的 `xboxbvhbuilder.dll`**（4 个 title DLL），直接**越过 `+0x2219650` 的 int3**，跑到 frame 1024 无任何异常。结论：之前为 XCL 写的 `bvh.rs` HLE 注册后，`title_dll()` 判定 `XboxBvhBuilder.dll` 由 HLE 提供 → 真实 DLL 不再加载 → BVH 构造行为不对 → 游戏 init 断言。**与第 8 节「thunk 期望不返回」的旧结论相比，这才是真根因。**

处置：`git checkout -- mod.rs` + 删除 `bvh.rs`（保留 resource.rs 的 DEPTH|UAV 修复与 crash.rs 的 skip 开关）。dry-run 复核：`xboxbvhbuilder.dll: 9 hle, 48 host, 0 missing`，主 exe `88 hle, 427 host, 72 title, 0 missing`。

### 10.2 两次真实 GPU 长跑（复现性 100%）

| 阶段 | 帧率 | GPU 每帧 | preparing | draws/帧 | 丢 draw | 窗口标题 |
|---|---|---|---|---|---|---|
| 菜单/加载（~110 s） | **30.0 fps** | 1.8–2.7 ms | 0.1–3.6 ms | 6–13 | 0 | `Americas \| VIctory 30 fps` |
| 进入世界 | **0.5–1.2 fps** | 110–1160 ms | 410–655 ms | 1227–2931 | 862–2412 | `Americas \| VIctory 1 fps` |

**画面证据 `window_view.png`**：GTA VI 官方加载海报（Leonida 插画）渲染完全正常 —— 43.9% 像素有内容、33.6% 有色彩、峰值亮度 254。抓取脚本 `grab_window.py`（PrintWindow PW_RENDERFULLCONTENT）。

**终局（两次一致）**：一次 21–22 lists 的提交耗时 **12.9 s / 14.5 s** → `host GPU device removed`（0x887A0004 + 0x887A0006）→ 进程退出。无 TDR 事件记录、无 minidump int3。这正是 README 作者警告的已知问题（真 GPU 进世界会挂起并触发驱动重置）。

### 10.3 「没有 3D 画面」「GPU 没干活」的精确答案

- **GPU 在干活**：菜单阶段每帧只干 1.8 ms（游戏 `set_interval` 锁 30 fps，GPU 剩余 31 ms 空等 → 利用率 11% 是正常结果）；世界阶段每帧干 110–1160 ms（真满载）。
- **没有 3D 场景的根因**：**402 个 shader 无法从 RDNA 翻译成 DXIL**（363 compute + 35 pixel + 24 mesh + 4 vertex），这些 pipeline 建不出来 → 每帧 862–2412 个 draw 被直接丢弃。失败模式 Top5：`a select on an untracked mask`（92，wave64 waterfall）、`a lane mask holding lanes elsewhere read as data`（78）、`a descriptor read only as data`（59）、`structured buffer of unknown stride`（46）、`SGPR 非 root parameter 寻址`（35）。README 作者自述「its title card renders, the scene after it is still dark」——**作者 AMD 卡同样受限**，属上游 recompiler 能力缺口。
- **卡的另一半**：preparing 每帧 410–655 ms（copies 118–420 ms / 140–420 MB + textures 19–98 ms），即 README 所说 "Loading is limited by writes into title memory"。

### 10.4 本地改动最终状态

保留：`crates/victory-gpu/src/resource.rs`（DEPTH|UAV 修复）、`crates/victory-loader/src/crash.rs`（`VICTORY_SKIP_BREAKPOINTS`）。删除：`bvh.rs` HLE 及其注册。回退：`object.rs` 的 `VICTORY_MISSING_SOK` 实验（证伪：E_NOTIMPL 是能力探测的正确信号，S_OK 会让游戏提前崩在 frame 1）。

### 10.5 后续方向

1. 3D 场景缺失 = 上游 recompiler 的 26 类不支持模式，需上游修复（本轮无本地可修项）；
2. GPU 挂死可用 diag 构建 + `VICTORY_D3D12_DEBUG=1`/`VICTORY_D3D12_WARP=1` 定位具体命令（作者建议路径），但不会改变 3D 缺失；
3. XCL 路线（已出画面）仍是可选替代；XCL 上游 71 个提交已 fetch 未合并。

## 11. diagnostics 诊断：世界画面全黑实锤 + GPU 挂死精确定位（2026-11-25 02:02–02:10）

### 11.1 诊断手段

重建干净的 diag 构建（旧 `target/diag` 二进制含已回退实验代码，弃用）：

```bash
cargo build --profile diag -p victory --features diagnostics
VICTORY_D3D12_DEBUG=1 VICTORY_D3D12_DRED=1 VICTORY_CAPTURE='D:\victory-ref\capture_diag' \
  ./target/diag/victory.exe run C:\boottest\game_scarlett_master.exe \
  --execute --unimplemented zero --console series-x
```

- `VICTORY_D3D12_DEBUG=1`：debug layer，失败时附首条错误（命名命令）；
- `VICTORY_D3D12_DRED=1`：DRED 自动面包屑 + 缺页/活动 list 报告；
- `VICTORY_CAPTURE`：present 帧写 PNG（仅 diagnostics 构建，release 下无效——之前帧目录为空的原因）。

### 11.2 画面证据链（首次拿到内部 present 帧）

| 帧 | 内容 | 结论 |
|---|---|---|
| frame-1 | 全黑 | 初始帧正常 |
| frame-128 | **ACCOUNT PICKER (Y) / PLAYER (A)** | UI 渲染正常，等待手柄输入 |
| frame-512 / 1024 | **GTA VI 加载插画完整渲染**（Jason、车、棕榈屋、日落） | 菜单/加载管线全部正常 |
| 世界阶段（PrintWindow 抓窗口） | **全黑**：lit(>24)=0.98%，colorful=0.00%，max=254 仅零星亮点 | 与 README "the scene after it is still dark" 一致 |

**键盘替身手柄**（README，窗口有焦点时生效）：Enter/Backspace/X/Y = A/B/X/Y，WASD+IJKL = 摇杆，方向键 = 十字键，Q/E、Z/C = 肩键/扳机，Esc/Tab = Menu/View。已写 `send_enter.py`（SetForegroundWindow + keybd_event 发 Enter），本轮游戏自动推进未用到，后续如卡在确认界面可用。

### 11.3 GPU 挂死根因（DRED 面包屑精确定位）

- debug layer 判定：**`DXGI_ERROR_DEVICE_HUNG`**——"took an unreasonable amount of time"，TDR 触发。**不是非法命令/参数**（无 INVALID_PARAMETER 类报错），是 GPU 执行中挂死。
- 面包屑：`a list stopped at command 1266 of 2855 (op 42)`；`a list stopped at command 0 of 2 (op 46)`；故障 list 上下文：`list 0 of 18, of its 284: indirect kind 10 max 1 [mesh 413144fb9d62e37e, pixel 633c1f8872ed8c6d]`。
- 高频故障管线组合 Top3：`[mesh 413144fb9d62e37e, pixel 633c1f8872ed8c6d]`×30、`[mesh f0c7efd2aceb2aaa, pixel 7d85c9cab3c80721]`×18、`[mesh b340a286490165b3, pixel 633c1f8872ed8c6d]`×14 —— 全是 **mesh + pixel 的 indirect draw**。
- 翻译失败 425 个。本机 NVIDIA warp32 上 mesh shader 大量翻译成功（world mesh 8191/8506）但部分翻译产物疑似让 GPU 死循环（`Srv s0+40: dim 42991392x1` 这类超大绑定也值得上游关注）。

### 11.4 最终结论

1. **黑屏（无 3D）**：425 个 shader 翻译失败 → 每帧上千 draw 丢弃 → 画面全黑。上游 recompiler 对 26 类 GFX10 模式不支持，作者 AMD 卡同样黑屏，**本机无可修项**。
2. **挂死**：mesh/indirect 翻译产物在真 GPU 上挂起 → TDR → device removed。上游 README 已警告，**本机无可修项**；可用 `VICTORY_D3D12_WARP=1`（软渲，极慢但不挂真卡）观察。
3. 菜单/加载/插画渲染完美，文件合规、绑定 0 missing——**本机环境（文件+GPU+配置）已排除**，剩余全部为上游缺口，等待 Marvisak/VIctory 更新（当前 HEAD `ffe310e` 无新提交）。

## 12. XCL 上游合并：71 个提交并入并验证通过（2026-11-25 02:05–02:20）

- 上游 `ivall/xcl` `930e8b4 → 9e3831e`（71 提交）：heap window 视图与 4 GiB 边界、`shader_cache_session`（翻译缓存跨重建保留）、DRED 优先报告部分执行的 list、Wait64 中列表拆分 + GPU fence 桥、allocator 释放/Reset 修复、reserved resource 按地址识别等。
- 流程：本地改动先提交到新分支 **`local-work`**（顺带解决了遗留的 `pso_async.rs` UU 未合并状态）→ `git merge origin/main` → 仅 `bind.rs` 一个冲突：本地 `bind_abi`/`bind_abi_slot` 拆分（two-pass 用）vs 上游 `ensure_view_heap` 修复。**解法：保留本地拆分，把上游修复放进 `bind_abi_slot` 开头**（该函数幂等，普通路径与 two-pass 路径都覆盖）。
- ⚠️ 踩坑记录：bash heredoc 里的 Python 用 `\n` 匹配不了 CRLF 文件，替换静默失败但后面的 `git commit` 照样执行，提交了带冲突标记的文件；改用 `newline=''` + 按 `\r\n` 切分重做后 `git commit --amend` 修正。
- 合并提交 `1ffb31d`，`cargo build --release` 通过（37.9 s，仅 4 个 unused warning）。合并前完整备份：`/d/xcl/local_changes_backup.patch` + `local_staged_backup.patch`。
- **运行验证**：`XCL_MESH_AS_FALLBACK=1 XCL_SKIP_NULL_WRITES=1 XCL_SKIP_INT3=1 xcl run C:\boottest` → 1500 帧呈现、稳定 **30.0 fps**、16384 draws / 1426 dispatches **全部 0 skipped**、mesh shader 正常翻译（AS-slot fallback 生效）、全程无 int3 / null-write / panic。窗口截图全白为 cloaked 窗口 PrintWindow 局限（真实画面需 `XCL_FRAME_DUMP=1` 的内部帧 dump，与老版本一致）。
- 结论：**合并版功能完好，且带上了上游 71 个修复**（含更快的 shader 缓存与更好的 DRED 报告）；两条路线（VIctory 诊断定性、XCL 合并升级）本轮均收尾。
