# ZJOU-VICTORY 性能框架与项目总览（详细版）

> 更新：2026-11-25 · 仓库：`github.com/FQY55kai/ZJOU-VICTORY`（HEAD 67076c7）
>
> **演示视频（本项目实况）**：[victory-amd 的英伟达移植，大部分着色器正常运行，目前已实现稳定的 3-4 帧](https://www.bilibili.com/video/BV14WyBzjEBW)（哔哩哔哩，短链 https://b23.tv/MJZBC3R）

---

## 一、项目定位与目标

**ZJOU-VICTORY** 是 Xbox Series 版 GTA6 在 Windows PC 上原生运行的翻译层（基于 VIctory 系开源工程），本项目聚焦**性能优化**：在其他分支负责渲染正确性的前提下，专攻帧率、资源占用与稳定性。

### 目标

| 层级 | 目标 | 现状 |
|---|---|---|
| 已达成 | 突破 30fps 节奏锁 | ✅ 开场 188-240fps（4-5ms/帧） |
| 已达成 | 世界加载 2fps → 稳定 4fps | ✅ 3.9-4.5fps（223-254ms/帧），零崩溃 |
| 已达成 | 静默退出修复（dstorage 竞态） | ✅ joshuu 修复已合入 |
| 近期 | 低分辨率降载功能修复（现默认关） | 用 D3D 调试层定位非法调用 |
| 近期 | 统一内存路径（VICTORY_UNIFIED）改造后默认开 | 对比/取回 pass 不再对共享内存 compute 哈希 |
| 中期 | 世界内 8-10fps、heap pass 406ms 尖峰消除 | 增量化描述符堆更新 |
| 远期 | 60fps 稳定、序章可玩、光影正确（当前阴影全黑：光线追踪光照未实现） | 依赖社区正确性进展 |

---

## 二、所用机器

### 主力测试机（本文全部实测数据来源）

| 部件 | 型号 |
|---|---|
| 整机 | 机械师（MACHENIKE）曙光16 Pro **L16P** |
| CPU | Intel Core **i9-13900HX**（24 核 32 线程） |
| GPU | NVIDIA **GeForce RTX 4080 Laptop**（12GB GDDR6） |
| 驱动 | NVIDIA **32.0.15.9733**（2026-11-25 更新） |
| 内存 | **31.7GB** DDR5 |
| 系统 | Windows 11 x64 |
| 游戏 | GTA6 Xbox Series 版（`game_scarlett_master.exe`，Series X profile，TitleID 7952987A），本地 dump（`C:\boottest`） |

### 引用项目的测试机（他人实测，用于对照）

| 项目 | 测试机 | 实测 |
|---|---|---|
| JoshuuDev/victory-amd | AMD RX 7800 XT 16GB + Ryzen 7 7700X + 32GB；另有 i7-14700KF + RTX 4070 12GB | 序章过场+第一房间 2-3fps，8 分钟基本不崩 |
| Zettrox/ztrx | NVIDIA RTX 3050 8GB | 序章从囚室到快艇追逐可玩，1-3fps，单次 1.5 小时不崩 |
| fxceless/victory-fork | NVIDIA RTX 4070 | 过场+3D 可渲染（社区排行 78%） |

---

## 三、引用项目（全部上游与参考工程）

### 直接血缘

| 仓库 | 角色 | 我们取用的内容 |
|---|---|---|
| **Marvisak/VIctory** | 一切的基座（loader/HLE/D3D12 后端/着色器重编译） | 全部架构；GPL-3.0 |
| **fxceless/victory-fork**（=Hyphen，社区排行 78%，开源第 2） | 主要上游 | `rect_list_gs`（矩形列表进几何阶段，治黑屏）；`e9765a7` wave64 挂死修复；纹理上传队列 310c6eb；depth bounds test + 快速默认 c96b4f2（#82 by Kaivyyy）；索引纹理复查 a803c87；AMD view heap clamp 31714de（trippywhale）；fx PR 移植 3bff182（asdaasdsaddf） |
| **JoshuuDev/victory-amd**（fork 自 myokex/victory-amd，再 fork 自 fxceless） | 2026-11-25 合并（e6a5ed7） | **dstorage 守卫竞态修复**（治静默退出）；页面全视图守卫（VICTORY_VIEW_GUARD，源自 victory-fork #115 by Spexivus + 修正 Spexivus/victory-fork#1 by joroyt + VIctory #1122 by tightfall）；纹理真内存 spans（placed_bytes_from）；dispatch >65535 组分部；dstorage page array 运行时读取；VICTORY_HEAP_RECHECK；**`victory warm` 着色器预热**；early pipeline list 跨运行保留；AMD view index 钳制 |
| **Zettrox/ztrx** | 参考 + 启动器 | `play.ps1` 启动器模式、README 状态基准；（其树=fxceless 落后 5 提交+启动器，无独特技术） |

### 对照与参考（未合并，仅学习/对比）

| 仓库 | 用途 |
|---|---|
| **JMAN730/victory** | 高频提交；shader 修复（v_permlanex16_b32 等）定点移植；全量合并 49k 行两次放弃 |
| **xcl-dev/xcl** | 行为对照（XCL：动画流畅但 3D 场景 35119 pipelines 堆积不消减） |
| **myokex/victory-amd** | joshuu 的直接上游 |
| **gta6recomp.com / PortWatch VI 排行榜** | 进度基准：omonkey 79%（私有）、Hyphen=fxceless 78%、VIctory 70%、AnyXboxX 65%、XCL 63%、Xbox2Windows 48%、Palmetto 25% |

### 四树复现对照结论（2026-11-25 上午）

| 树 | 状态 |
|---|---|
| fxceless/victory-fork | 唯一能渲染过场+3D（后成为我们基底） |
| JMAN730/victory | 黑屏（present 合成断），shader 修复可取 |
| Marvisak/VIctory | 菜单 20fps 后 GPU 设备丢失（缺 wave64 修复） |
| xcl-dev/xcl | 动画流畅但 3D 场景加载不出 |

---

## 四、达成的情况（实测数据）

### 4.1 30fps 突破（帧调度）

- **根因**：标题用 Xbox 帧间隔机制 `set_interval(33.333ms)` 在帧时钟上阻塞定步；渲染本身仅 4-5ms。
- **实现**：`VICTORY_FRAME_INTERVAL=<µs>` 覆盖（`0`=uncapped 500µs tick）+ `VICTORY_VSYNC=0`。
- **实测**：开场动画 **30fps → 188-240fps（4-5ms/帧）**。
- 注意：uncapped 后出现"走一步退一步"现象（游戏帧步进被破坏），正式游玩建议 `VICTORY_FRAME_INTERVAL=16667`（60Hz）。

### 4.2 世界加载段性能（433ms → 223ms，2.3fps → 4.5fps）

| 指标 | 今晨基线 | 现在（67076c7） | 手段 |
|---|---|---|---|
| 帧时间 | 433ms | **223-254ms** | 以下全部叠加 |
| 帧率 | 2.3fps | **3.9-4.5fps** | |
| fetch 等卡 | 262ms | **8.6ms** | 卡上拷贝优先（e2852a6） |
| copy back | 328ms | **12.7ms** | 同上 + WIDE_MERGE 默认开 |
| 卡上缓冲拷贝 | 1 个 174MB | **97 个 941MB** | `VICTORY_CARD_ROOM_PERCENT=95`（拷贝 95% vs 纹理 90%） |
| 纹理刷新 | 1029ms（等重映射 963ms） | **3.8ms** | 上游 310c6eb/a803c87 + WIDE_REFRESH |
| CPU 快照 | 689MB/帧 | 随拷贝上卡大幅下降 | 同 e2852a6 |
| hash 对比遍数 | 121-146 遍 | 合并运行（compare 15→4） | `VICTORY_CARD_REHASH_RUNS` 默认开 |
| 着色器翻译失败 | 13 条 | **2 条** | 我们的 descriptor/f64/interp 修复 + 上游 |

### 4.3 稳定性

- **静默退出修复**：dstorage 守卫竞态（标题世界第一帧后数秒自停 2/2 复现）——joshuu `10ea38e` 修复已合入。
- **`victory warm`**：着色器预热命令（约 4 分钟），进游戏不再缺物体/首分钟卡翻译。
- 世界加载段连续运行零设备错误（LOWRES=0 路径）。

### 4.4 实验性功能（默认关，有明确方向）

| 开关 | 状态 | 发现 |
|---|---|---|
| `VICTORY_UNIFIED=1` | 默认关 | 统一内存映射（CUSTOM heap/L0/WriteCombined ≈ Resizable BAR 共享内存）：UPLOAD heap 不许 UAV；L0 可用但现有 compute 哈希压共享内存 1 分钟内挂设备 → **改造方向明确**：对比/取回 pass 用 CPU 侧或写标记替代 compute 哈希后可默认开，届时 271ms/帧的"标题等内存读"串行点消失 |
| `VICTORY_LOWRES_MIPS=1~8` | 默认 0 | 显存满时纹理丢最大 mip（降分辨率不等待）：drop>0 时设备挂死（无效调用，两次复现）→ 待 D3D 调试层（`VICTORY_D3D12_DEBUG=1`）定位 |
| `VICTORY_FRAME_INTERVAL=0` | 可用 | 突破 30fps；游戏有第二层限速与步进依赖，60Hz 更稳 |

### 4.5 现存资源大头（世界加载段 223ms 帧构成，下一步目标）

| 大头 | 消耗 | 对策 |
|---|---|---|
| copies 上传 | 35ms/205MB 每帧 | JOIN_COPIES 已开；考虑增量 |
| heap pass 尖峰 | 12-406ms | 描述符堆增量更新（待做） |
| GPU 渲染 | 随场景波动 | 依赖正确性分支（5024 draws/帧） |
| 纹理流送队列 | 1.8-2.4 万 slot 在途 | 已有 catchup 机制 |

---

## 五、自研改动清单（相对全部上游）

| 提交 | 内容 |
|---|---|
| `11cdbec` | shader：bindless image 尺寸重建（descriptor_words/getDimensions）、v_cvt_f32_f64、插值修复；vsync 默认开、纹理预算 |
| `fcd3584` | 合并 fxceless 5 新提交 |
| `4386f55` | 项目更名 ZJOU-VICTORY；**VICTORY_FRAME_INTERVAL** 帧间隔覆盖 |
| `d741399` | REHASH_RUNS/JOIN_COPIES/WIDE_REFRESH/WIDE_MERGE **默认开**（纯并行无正确性风险） |
| `e2852a6` | **卡上拷贝优先于纹理**（95% vs 90%，VICTORY_CARD_ROOM_PERCENT） |
| `6c79025` | 低分辨率纹理（VICTORY_LOWRES_MIPS，现默认 0） |
| `e6a5ed7` | 合并 JoshuuDev/victory-amd（10 处冲突，Spans 取真内存方案） |
| `bfdb186` | 合并后续：rechecks 记账、双 recheck 共存、depth-bounds 命名 |
| `31904df`/`67076c7` | 低分辨率默认关（稳定优先） |

---

## 六、关键开关速查

| 开关 | 作用 | 建议 |
|---|---|---|
| `VICTORY_FRAME_INTERVAL` | 覆盖帧间隔（µs） | 游玩 `16667`；测峰值 `0` |
| `VICTORY_VSYNC` | 垂直同步 | 测峰值 `0` |
| `VICTORY_CARD_ROOM_PERCENT` | 卡上拷贝显存份额 | 默认 `95` |
| `VICTORY_CARD_REHASH_RUNS` | hash 遍合并 | 默认开 |
| `VICTORY_JOIN_COPIES` / `VICTORY_WIDE_REFRESH` / `VICTORY_WIDE_MERGE` | 并行/合并 | 默认开，`=0` 退回 |
| `VICTORY_LOWRES_MIPS` | 显存满丢 mip | **默认 0（勿开，待修）** |
| `VICTORY_UNIFIED` | 统一内存缓冲 | **默认 0（勿开，待改造）** |
| `VICTORY_HEAP_RECHECK` / `VICTORY_VIEW_GUARD` / `VICTORY_TEXTURE_SWEEP` | joshuu 修复开关 | 默认开 |
| `victory warm` | 着色器预热 | 更新构建后跑一次 |

---

## 七、已知问题

1. **阴影全黑**：光线追踪光照/阴影未实现（全社区一致，非本项目范围）。
2. **走一步退一步**（uncapped 帧率下）：游戏帧步进被破坏；60Hz 下验证中。
3. **低分辨率功能挂设备**：任何 mip 丢弃触发"无效调用"设备移除，已默认关闭。
4. **统一内存路径挂设备**：compute 哈希压共享内存所致，改造后启用。
5. **帧抖动（一顿一顿）**：heap pass 406ms 尖峰 + 流送波动。

---

## 八、招募：真 RT GI（光线追踪反弹光照）—— 需求与步骤

**问题**：人脸与室内是黑的。截图证据：贴图、直射阳光全部正常（Lucia 头发/衣服材质完好），但阳光照不到的地方（脸的背光面、室内）几乎纯黑——缺少光追反弹光照（RTGI）。这是全社区（VIctory / victory-fork / victory-amd 各 README 一致承认）都缺失的最后一块大拼图：**`ray-traced bounce light does not run`**。

**为什么它值得做**：这是从"能跑"到"好看"的分水岭。做完之后阴影不再是死黑，画面观感接近主机。

### 我们已经有的（地基是实的）

| 组件 | 状态 |
|---|---|
| `XboxBvhBuilder.dll` 原生运行 | ✅ BLAS/TLAS 构建在跑（标题的 BVH 数据包 `core_bvh4*.rpf` 经 DirectStorage 正常加载） |
| BVH 内存布局 | ✅ 标题把 BVH 放在自己内存里（xmem 的地址范围约束正是为它：`the title's BVH builder refuses one above`） |
| 描述符"只取地址"路径 | ✅ 翻译器已有：`a buffer view the code only takes the address of, to walk the memory behind it (a ray-tracing structure)`——识别出了 BVH 遍历着色器 |
| 双精度/波求和 | ✅ phase2 已支持（水/大气的透射率、体积计算） |
| 部分光照累积着色器 | ✅ 已能翻译（deferred 面、水洼、浅水） |

### 缺什么（按证据推断，从易到难）

**A. 先搞清楚"RT"在主机上的实现形态（1-2 天，最关键的第一步）**

Xbox 的 XDXR 有两种可能的实现，决定了完全不同的工程路线：
1. **着色器软件遍历**：着色器自己用 buffer 读走 BVH 树（我们已看到这类着色器的迹象——"拿 buffer 地址走内存"的模式）。若是这样，工程量在着色器翻译修复。
2. **硬件 TraceRay**：`TraceRay/ReportHit/AcceptHit` 等 DXIL 光线内建。若是这样，需要 RTPSO 全套（见 B）。

判别方法：
- 开诊断构建跑世界场景，收集**未翻译/被丢弃的着色器**清单（现有日志直接给）；
- 对每个失败着色器跑 `VICTORY_DESC_WHY=1`，看是否大量出现"描述符只取地址"（软件遍历）；
- 顺带把仅存的 4 个翻译失败（`whole-wave section step` ×2、`lane mask exchanged`、`descriptor @0x38`）解剖——它们疑似就是遍历循环。

**B. 若是硬件 TraceRay（1-2 周）**

1. `D3D12_STATE_OBJECT`（RTPSO）创建翻译：DXIL library / hit group（closest-hit、any-hit、miss）/ shader config / pipeline config / global root signature 子对象；
2. `DispatchRays` → `DispatchRays` + SBT（Shader Binding Table）布局还原（标题自建 SBT 的结构要逆向它的 record 布局）;
3. DXIL 光线内建的翻译：`TraceRay`（参数包：加速结构句柄、ray mask、contribution、miss index、payload…）、`ReportHit`、`IgnoreHit`、`AcceptHitAndEndSearch`、`RayQuery_*`（inline 遍历）；
4. **加速结构视图**：`D3D12_RAYTRACING_ACCELERATION_STRUCTURE` SRV（描述符 words[3] 是 AS 的 GPU VA）；
5. **最大的坑——AS 格式转换**：XboxBvhBuilder 产出的是 **XDXR 格式 BVH**，PC DXR 只吃标准 AS。要么写 XDXR→DXR AS 转换器，要么从几何重建 AS（标题的 BVH 叶子里有几何）。

**C. 若是软件遍历（3 天-1 周）**

1. 修复 4 个现存翻译失败（遍历循环的 whole-wave 模拟、lane mask）；
2. 描述符地址读取（`0x38` 偏移）的数值化；
3. 遍历循环的性能（BVH 走树是长循环——现有的 loop-limit/波模拟要能扛住）。

**D. 光照累积（共通，1-3 天）**

反弹光的累积 pass（间接光求和、环境光探针）——phase2 的双精度波求和正是它的底座。

### 验收标准

- 序章囚室截图：Lucia 脸的背光面、背景人物、室内墙壁**不再是纯黑**（有可辨的环境/反弹光）；
- 帧率不低于当前基线（3fps 起步，预期 RT 开销可控在 2 倍内）；
- 连续运行 10 分钟无设备丢失。

**如果你要做这个，从 A 开始，把 A 的结论（软件遍历 or 硬件 TraceRay）发 issue，再动手。** 欢迎认领。

---

## 构建与运行

```powershell
# 构建（需 Rust 工具链、MSVC C++ 工作负载、Windows SDK）
cargo build --release -p victory

# 着色器预热（推荐，约 4 分钟；更新仓库后重跑）
target\release\victory.exe warm

# 运行（游戏为自备的 Xbox Series 版 dump）
target\release\victory.exe run "<游戏目录>\game_scarlett_master.exe" --execute --console series-x
```

节奏与开关：`VICTORY_FRAME_INTERVAL=16667` 为 60Hz 节奏（省略则 30fps），`VICTORY_VSYNC=0` 关垂直同步；完整开关见上文速查表。缓存与存档默认在系统的本地应用数据目录下 `victory` 文件夹，`scripts\play.ps1` 为一键启动器。

**本仓库不含任何游戏文件、代码或资源**，需要自备游戏副本；与游戏开发/发行方无关。
