# ZJOU-VICTORY

**在 Windows PC 上原生运行 Xbox Series 版游戏的翻译层——本仓库只做一件事：把帧率压下来。**

基于 VIctory 系开源工程。渲染正确性由上游与兄弟分支负责，这里专攻性能、内存占用与稳定性，
并且要求**每个结论都有可复现的测量**：测量方法见 [`docs/perf.md`](docs/perf.md)，
对照脚本见 [`tools/bench/compare.py`](tools/bench/compare.py)。

> 本仓库不含任何游戏文件、代码或资源。需要自备依法获得的副本；与游戏开发/发行方无关。
> 许可继承上游 GPL-3.0（见 [LICENSE](LICENSE)）。

---

## 1 问题的形状

游戏是为**一台统一内存的主机**写的：CPU 与 GPU 看同一份字节，shader 用 GPU 虚拟地址直接读描述符，
绘制包由硬件的 command processor 直接消费，波宽 64 车道。PC 上这四件事一件都不成立。

于是每一层都要替硬件做一遍它本来免费做的事：

| 主机上免费 | 移植后要谁付 |
|---|---|
| 统一内存 | 标题内存 + 宿主副本 + 卡上副本，**三份**，每帧往返上百 MB |
| 硬件消费绘制包 | CPU 逐条重放，4,000–6,000 条绘制/帧 |
| bindless 按地址读描述符 | 每帧重建描述符表，**上万张** |
| 64 车道波 | D3D12 下只有 32，wave64 程序拆两半再用 LDS 交换 |

**这就是为什么移植版比原生贵。差距不是某个系数，是这四项各自乘上去的结果。**

## 2 目标

| 层级 | 目标 | 状态 |
|---|---|---|
| 已达成 | 突破标题的 30 fps 节奏锁 | ✅ 开场 188–240 fps（4–5 ms/帧） |
| 已达成 | 世界加载段 433 ms → 200 ms 以内 | ✅ 172–254 ms（视场景 4,400–6,600 draws） |
| 已达成 | 长时间运行零设备错误 | ✅ 静默退出（dstorage 竞态）已修；`victory warm` 预热 |
| **本轮** | **建立可信的性能测量，找出唯一值得开的开关** | ✅ 噪声带量出来了，`VICTORY_INPLACE` **−26%** |
| 近期 | 世界内稳态 20 fps 以上 | 68.6 ms/帧（Series S 快路）；Series X 快路仍在 150 ms 以上 |
| 中期 | 消除 heap 遍历的 400 ms 级尖峰（开局"一顿一顿"） | 已定位到装饰烘焙阶段的按需贴图上传 |
| 远期 | 60 fps 稳定、光影正确 | 阴影全黑＝光线追踪光照未实现，见 [`docs/rt-gi.md`](docs/rt-gi.md) |

## 3 现在在干什么

**把"感觉快了"换成"量出来了"。** 一个开关到底有没有用，过去只能靠单轮 fps 的感觉；现在这件事有判据。

上游自带帧时间转储（`VICTORY_FRAMETIME_DUMP`，37 列 CSV，表头带机器与全部开关）。用它做两轮
**同配置对照**（A/A），先量出这台机器的噪声：

| 量 | A/A 两轮之差 |
|---|---|
| 字节类计数器（`copies_mb`、`copyback_mb`、`card_ranges`…） | **±0.4% ~ ±4.8%** |
| 帧时间 `ms_per_frame` | **±9.2%** |
| 按场景指纹配对的逐对差 | 中位 −0.9 ms，**均值 +26.9 ms**，52% 的帧"变快" |

结论：**计数器可信，帧时间不可信。** 判据因此是——效应 > 20% 才认，低于 20% 只能说"未测出"。
单轮 fps 对比，包括任何 README 里写的数字，都是这个量级的东西。

## 4 已经达成

### 4.1 稳态一帧的构成（Series S 快路，实测）

| 项 | 值 |
|---|---|
| 帧时间 | **68.6 ms**（14–15 fps） |
| `preparing` | 47.4 ms（其中内存拷贝 15.1 ms / 81.9 MB、贴图 6.4 ms、堆遍历 9.1 ms） |
| `copying back` | 22.7 ms / 46.1 MB |
| GPU | **0.0 ms** |

GPU 全程空闲——**瓶颈 100% 在 CPU 侧的搬运与记账**。逐项账本与三项每帧 O(千) 的扫描
（描述符表 11,380 张/帧、卡上拷贝 24,205 段/帧、堆遍历 10,134 槽/帧）见 [`docs/perf.md`](docs/perf.md)。

### 4.2 唯一超过噪声带的开关：`VICTORY_INPLACE=1`

把标题窗口所在的私有内存直接开成 host heap、在上面放 buffer——**标题的字节就是 shader 读的字节**，
不再有副本。

| 量 | 变化 |
|---|---|
| 帧时间（2 轮开 / 2 轮关） | 中位 **98.5 → 73.2 ms（−26%）** |
| `copies_mb` | −16.5% |
| `textures_mb` | −69% |
| `copyback_mb` | −38.6% |

覆盖面是它的上限：只放下 **144 窗口 / 206 MB**，另有 **5,732 个**被拒——这个标题的窗口大多落在页池的
section 视图里，那种内存开不成 host heap。全量 in-place 需要 level 2，而它不能与 late wait 同用，
`VICTORY_RELAY` 隐含 late wait 3。**这是下一步最值钱的一件事。**

### 4.3 试过但无效的（记下来，省得再试）

| 想法 | 实测 |
|---|---|
| `VICTORY_UM_COALESCE_KB=64` 合并上送区间 | 区间 −91%，但上送字节 **+76%**，帧时间未测出差异：多出的字节就是被填掉的间隔本身 |
| `VICTORY_UNWATCHED_FRAMES=2` 隔帧整块拷贝 | 拷贝量未降（68.1 vs 70.3 MB），无效 |
| `SetVideoMemoryReservation` 抬显存预算 | 预算纹丝不动（那是下限，不是上限），帧率无变化 |
| `VICTORY_TEXTURE_ROOM_PERCENT=94` | 更慢约 25 ms/帧：显存用得多 → 每帧搬得更多 |
| 用 `NtReadVirtualMemory` 替 `ReadProcessMemory` | 无意义：Vista 之后前者就是后者的实现 |

### 4.4 稳定性

- **静默退出**（世界第一帧后数秒自停）：dstorage 守卫竞态，上游 `10ea38e` 已修
- **`victory warm`**：着色器预热（约 4 分钟），进游戏不再缺物体、首分钟不卡翻译
- 世界加载段连续运行零设备错误

## 5 方向

1. **上游 GPU 侧重构那 85 个提交**——页标记回读、单视图共享大拷贝缓冲、堆同步只处理变动槽位、
   纹理暂存池。这是唯一能动"命令录制 + 内存往返"两项成本的东西，代价是一次 400+ 冲突的合并。
2. **`VICTORY_INPLACE` 全量**：已证 −26%，卡在 level 2 与 late wait 的互斥上。
3. **真 RT GI**：阴影全黑是缺整套光线追踪反弹光照，需求与步骤见 [`docs/rt-gi.md`](docs/rt-gi.md)。
   上游的着色器层早就在我们树里，缺的是一道硬门（53 个 builder 程序里 3 个翻译不出）。

## 6 引用的项目

### 直接血缘

| 仓库 | 角色 | 我们取用的内容 |
|---|---|---|
| **[Marvisak/VIctory](https://github.com/Marvisak/VIctory)** | 一切的基座：loader / HLE / D3D12 后端 / 着色器重编译 | 全部架构（GPL-3.0） |
| **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)** | 主要上游 | `rect_list_gs`（治黑屏）、wave64 挂死修复、纹理上传队列、depth bounds test、索引纹理复查、AMD view heap clamp |
| **[JoshuuDev/victory-amd](https://github.com/JoshuuDev/victory-amd)** | 2026-11-25 合并 | dstorage 守卫竞态修复、页面全视图守卫、纹理真内存 spans、dispatch >65535 分组、`victory warm` 预热 |
| **[Zettrox/ztrx](https://github.com/Zettrox/ztrx)** | 参考 + 启动器 | `play.ps1` 启动器模式与状态基准 |

### 对照与参考（未合并）

| 仓库 | 用途 |
|---|---|
| [JMAN730/victory](https://github.com/JMAN730/victory) | 高频提交；shader 修复定点移植（全量合并 49k 行两次放弃） |
| [xcl-dev/xcl](https://github.com/xcl-dev/xcl) | 行为对照 |
| [myokex/victory-amd](https://github.com/myokex/victory-amd) | joshuu 的直接上游 |
| [gta6recomp.com](https://gta6recomp.com) / PortWatch VI | 进度基准 |

### 方法与工具的出处

| 来源 | 用途 |
|---|---|
| 上游 `docs/unified-memory/`（01 现状 / 02 技术综述 / 03 设计） | 统一内存的完整成本地图。**注意：其中开关表的 code default 列已与源码不一致**（`wide_back()` 注释写 "On by default"，文档写 off），查之前要回源码 |
| shadPS4、yuzu、Ryujinx、Xenia、RPCS3 | 统一内存模拟的技术对照（同上文档 §2） |
| D3D12 / DXGI 文档，DirectX 12 Agility SDK | 主机堆、upload heap、`OpenExistingHeapFromAddress` |
| Rust · windows-rs · DXIL / FXC | 实现语言与着色器通道 |

## 7 复现

```powershell
# 构建（Rust 工具链 + MSVC C++ 工作负载 + Windows SDK）
cargo build --release -p victory

# 着色器预热（约 4 分钟；换仓库后重跑）
target\release\victory.exe warm

# 运行（游戏为自备的 Xbox Series 版 dump）
target\release\victory.exe run "<游戏目录>\game_scarlett_master.exe" --execute --console series-s

# 带帧时间转储，然后对比两轮
$env:VICTORY_FRAMETIME_DUMP = "D:\bench\a.csv"
python tools/bench/compare.py a.csv b.csv --label 基线 --label 改动
```

本仓库实测所用开关块见 [`scripts/run-perf.sh`](scripts/run-perf.sh)；`VICTORY_FRAME_INTERVAL=16667`
为 60 Hz 节奏（省略则 30 fps），`VICTORY_VSYNC=0` 关垂直同步。完整开关速查见
[`docs/switches.md`](docs/switches.md)。

### 测试机

机械师 曙光16 Pro **L16P** · i9-13900HX（24 核）· RTX 4080 Laptop 12 GB · 31.7 GB DDR5 · Windows 11 x64。
其余对照机见 [`docs/machine.md`](docs/machine.md)。

## 8 文档

| 文件 | 内容 |
|---|---|
| [`docs/perf.md`](docs/perf.md) | **性能账本**：怎么量、噪声带、一帧的构成、开关结论 |
| [`docs/changes.md`](docs/changes.md) | 相对上游的自研改动清单与代码结构 |
| [`docs/switches.md`](docs/switches.md) | 关键开关速查 |
| [`docs/rt-gi.md`](docs/rt-gi.md) | 真 RT GI 的需求、缺口与验收标准 |
| [`docs/upstream-sync.md`](docs/upstream-sync.md) | 上游同步流水账（逐条合入、冲突、分歧）——**记录，不是结论** |
| [`docs/machine.md`](docs/machine.md) | 测试机与对照机规格 |
| [`tools/bench/compare.py`](tools/bench/compare.py) | 两份帧时间转储的对比：计数器、噪声带、按场景配对 |
