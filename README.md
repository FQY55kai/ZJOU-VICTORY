# ZJOU-VICTORY — GTA VI (Xbox) → PC 实测配置包 + 启动器

> 基于 **victory-scarlett-3070**（"scarlett-fox" 分支）的 Xbox Series X|S 版 GTA VI PC 移植成品
> 在 **RTX 4080 Laptop 12GB** 上经 **run2–run9 共 8 轮实测**调优定稿：**21–30 fps**，序章可玩、正常退出
> 本仓库当前为**成品包 + 启动器**；完整源码见 git 历史 `6645043`

---

## 目录

- [包内容](#包内容)
- [快速开始](#快速开始)
- [原理](#原理)
- [实现状态](#实现状态)
- [资源利用情况](#资源利用情况)
- [与其余项目的对比](#与其余项目的对比)
- [排错速查](#排错速查)
- [引用与致谢](#引用与致谢)
- [免责声明](#免责声明)

## 实测环境

| 项目 | 配置 |
|---|---|
| CPU | Intel Core i9-13900HX（24C/24T） |
| GPU | NVIDIA RTX 4080 Laptop（12GB） |
| RAM | 32GB DDR5 |
| 游戏 | GTA6.Xbox.v1.0.2.1（`game_scarlett_master.exe`） |
| 验证日期 | 2026-11-27 |

## 包内容

| 文件 | 用途 |
|---|---|
| `Launcher.cmd` | **菜单启动器**（启动 / 预热 / 看参数 / 改游戏目录） |
| `victory.exe` | 主程序（scarlett-fox 分支编译产物） |
| `Start-4080-GTA6.cmd` | 一键启动（全部环境变量已按实测最优组合写死） |
| `Warm-shaders.cmd` | 着色器预热（**首次必跑**，约 21 分钟，只跑一次） |
| `使用说明.txt` | 快速上手（中文） |
| `配置文档.md` | 完整参数表 + 三轮实测排错记录 |
| `scarlett-fox-4080-20261127.zip` | 以上全部内容的打包 |

## 快速开始

```text
1. 双击 Launcher.cmd
2. 首次先选 2（预热，约 21 分钟，只跑一次）
3. 选 1 启动（着色器缓存热后约 10 秒进菜单）
```

---

## 原理

victory 系移植器把 Xbox Series X|S 游戏在 PC 上"就地翻译"运行，核心链路：

```text
Xbox 游戏进程（game_scarlett_master.exe）
  │
  ├─ 加载层  victory-loader：直接映射游戏 ELF/EXE，TLS 重定向（static TLS index 补丁 5 万+ 处），
  │           console 指令翻译、栈/线程托管、崩溃捕获
  ├─ HLE 层   victory-hle：宿主实现 Xbox OS 系统模块 —— xmem（物理内存镜像/软视图）、
  │           dstorage（DirectStorage 直通）、d3d12x、gameinput、xuser/xnetworking 等约 25 个模块
  ├─ GPU 层   victory-gpu + victory-shader：
  │           · 每帧把游戏提交的 D3D12X 命令翻译成 PC D3D12
  │           · 着色器 DXBC → DXIL 实时重编译（含 WAVELAYOUT / 波前重排、/resource 绑定降级）
  │           · 纹理流送：BCn 反 tiled + 增量上传（实测序章累计 214 万张 / 102GB）
  │           · 显存预算管理：TEXTURE_BUDGET_MB 硬顶 + ROOM_PERCENT 动态回收（防 Windows 驱逐显存）
  └─ 时钟层  帧率补丁：直接 patch 游戏读帧间隔的指令（VICTORY_FRAME_INTERVAL_MS）
```

**本配置包的三项关键机制**（均由实测事故反推得出）：

1. **防静默崩溃**：显存超预算时 Windows 会把 GPU 内存强制挪进 PC 内存，进程无任何日志死亡。
   源码注释原话 *"Past the budget the system moves the card's memory into the PC's, which ended runs 20 and 24"*。
   `VICTORY_TEXTURE_BUDGET_MB=6144` + `ROOM_PERCENT=85` 双闸门兜底。
2. **防 GPU 复制碎片化**：`CARD_BUFFERS_MB` 减半会使每帧 buffer copies 流量翻倍、GPU 时间从 <5ms 暴涨到 150ms/帧（帧率 26→5）。
3. **黑块 vs 帧率平衡**：`ROOM_PERCENT` 控制纹理驻留水位——80 帧率高但纹理被提前回收产生黑块，90 黑块消失但回收/重传吃掉帧预算，**85 为实测平衡点**。

## 实现状态

| 功能 | 状态 |
|---|---|
| 游戏启动 / 主菜单 / 世界加载 | ✅ 稳定 |
| 序章实机游玩（画面 + 输入） | ✅ 可玩，21–30 fps |
| 键盘映射（手柄键位全量） | ✅ 正常 |
| 着色器缓存持久化（39MB 管线库） | ✅ 二次启动加载提速 |
| 正常退出（title suspended） | ✅ exit 0 干净退出 |
| 60fps 档 | ❌ CPU 翻译层追不上（25–40ms/帧），锁定 30fps 档 |
| 存档兼容性 / 全流程通关 | ⚠️ 未验证（序章为止） |
| 手柄物理设备 | ⚠️ 键盘映射完整，物理手柄未测 |

调优过程问题全部闭环：run2 静默崩溃（显存超预算驱逐）、run3 4–6fps（CARD_BUFFERS 512 碎片化）、黑块（ROOM 80 回收过激）。

## 资源利用情况

run8/run9 实测（序章场景，连续采样）：

| 资源 | 数值 | 说明 |
|---|---|---|
| 显存占用 | 8.5–10.4GB / 12GB（70–84%） | budget 被 Windows 动态压缩过（11227→9927MB），room 回收规则兜底 |
| 显存回收 | 累计回收/重传 11GB+（23 万次） | 驻留与帧率的动态平衡机制 |
| GPU 利用率 | 脉冲式 0–99% | 翻译器按帧突发提交，非持续满载 |
| GPU 功耗/温度 | 42–78W / 50–56°C | 远低于功耗墙，2445MHz 满睿频 |
| CPU 整机 | 74%（24 线程全开） | **主要瓶颈**：命令/着色器翻译层 |
| victory 物理内存 | 10.4GB 工作集 / 27GB 提交 | 提交超物理 17GB 由页文件承载 |
| 系统剩余物理内存 | ~2.7GB | 偏紧；关后台应用可显著减压 |
| 帧率分布 | 30fps 跑满为主，场景切换掉至 13–22 后秒回 | 流送脉冲 |

## 与其余项目的对比

同机（4080 Laptop 12GB）环境下的定性对比（其余包以包内说明与公开说明为准，未做同帧率基准）：

| 项目 | 定位 | 显存策略 | 本机表现 |
|---|---|---|---|
| **scarlett-fox-4080（本包）** | RTX 3070 fork 定向调优，纹理降 mip + 预算双闸门 | MIP_DROP=1 + BUDGET=6144 + ROOM=85 | **21–30fps，序章可玩**（本机实测基准） |
| avatarmanz-v1（VIctory-7900XTX fork） | A 卡 16GB 优化（7900XTX 目标），N 卡可跑 | 原生 94% room，无硬顶 | 可启动；12GB 卡上显存余量不足，需自行加预算闸门 |
| cool_fox_dg | cool_fox 谱系，带 12GB / Xbox Series S 两套 profile | 分档 room | 12GB 档思路与本包一致，无黑块/崩溃双修复 |
| coolfox-20261127 | cool_fox + Magpie 超分 + 诊断工具 | 默认 | 附 victory-diag.exe 诊断器，适合排障 |
| PS5 原生（KytyPS5 + dump 路线） | SELF 加载 + libc NID 兜底 | — | 尚无法进入游戏（unresolved imports），落后于 Xbox 路线 |

**结论**：12GB 显存的 N 卡上，scarlett-fox 分支 + 本配置包的组合是目前唯一跑通"启动→序章可玩→正常退出"全链路的方案。

## 排错速查

| 症状 | 根因 | 处理 |
|---|---|---|
| 静默崩溃（日志无 ERROR、无 WER） | 显存超预算被系统驱逐 | 确认 `TEXTURE_BUDGET_MB=6144` 与 `ROOM_PERCENT=85` 存在 |
| 持续 4–6 fps，GPU >100ms/帧 | `CARD_BUFFERS_MB` 被改小（512 实测） | 回到 `1024` |
| 画面黑块 | 纹理被提前回收（ROOM 过低） | `ROOM_PERCENT` ≥ 85 |
| 帧率上不去且 GPU 时间 0ms | CPU 翻译瓶颈 | 关诊断统计（`TABLE_MEMO`/`QUEUE_MARKS`/`LIST_ALIAS_EXACT`）或 `MAP_THREADS=2` |
| Windows 事件查看器出现 `RADAR_PRE_LEAK_64` | 内存泄漏前兆 | 同防崩处理，减纹理预算 |

## 引用与致谢

- **victory-scarlett-3070 分支**：基于 victory 谱系（Marvisak/VIctory → fxceless → Op-lite/victory-scarlett）的 RTX 3070 定向 fork，本包在此之上做 4080 Laptop 12GB 实测调优
- **avatarmanz/VIctory-7900XTX**：A 卡优化分支，包内键位表与启动框架参考
- **cool_fox / cool_fox_dg**：显存分档（12GB/XSS profile）思路来源
- fork 源码 `docs/`（architecture / perf / switches / rt-gi）为本 README 原理节的依据，完整源码在 git 历史 `6645043`
- 实测数据：`C:\GTAVI\run\scarlett-fox-run{2..9}.log`（48MB+ 全量日志）
- 开发记录：https://github.com/FQY55kai/ZJOU_PS5toPC_GTA6 `docs/development-log.md`

## 免责声明

本项目仅用于**个人已合法拥有的游戏数据的互操作性研究与技术学习**，不分发、不包含任何游戏资产（ROM / 资源文件 / 密钥）。所有商标归各自权利人所有。
