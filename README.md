# ZJOU-VICTORY — GTA VI (Xbox) → PC 实测配置包

> 基于 **victory-scarlett-3070**（"scarlett-fox" 分支）的 Xbox 版 GTA VI PC 移植成品包
> 在 **RTX 4080 Laptop 12GB** 上实测调优，稳定 **21–26 fps**，含防崩溃与防卡顿的全部修复
> 本仓库当前为**成品包**；完整源码见 git 历史 `6645043`

---

## 实测环境

| 项目 | 配置 |
|---|---|
| CPU | Intel Core i9-13900HX |
| GPU | NVIDIA RTX 4080 Laptop（12GB 显存） |
| 游戏 | GTA6.Xbox.v1.0.2.1（`game_scarlett_master.exe`） |
| 实测性能 | **21–26 fps**（38.4 ms/帧），GPU 时间 <5 ms，瓶颈在 CPU 翻译层 |
| 验证日期 | 2026-11-27（run5） |

## 包内容

| 文件 | 用途 |
|---|---|
| `victory.exe` | 主程序（scarlett-fox 分支编译产物） |
| `Start-4080-GTA6.cmd` | 一键启动（全部环境变量已按实测最优组合写死） |
| `Warm-shaders.cmd` | 着色器预热（**首次必跑**，约 21 分钟，只跑一次） |
| `使用说明.txt` | 快速上手（中文） |
| `配置文档.md` | 完整参数表 + 三轮实测排错记录 |
| `scarlett-fox-4080-20261127.zip` | 以上全部内容的打包（方便转存） |

## 快速开始（三步）

```text
1. 记事本编辑 Start-4080-GTA6.cmd
   把 set VICTORY_GAME_DIR=C:\GTAVI\GTA6.Xbox.v1.0.2.1 改成你的游戏目录

2. 双击 Warm-shaders.cmd（仅首次，约 21 分钟）

3. 双击 Start-4080-GTA6.cmd 启动
```

## 启动过程预期（重要）

| 阶段 | 表现 | 说明 |
|---|---|---|
| 开场菜单 | 跑满 30 fps | 正常 |
| 世界加载流送期 | 4–17 fps，持续 1–3 分钟 | **13 万张纹理上传 + GPU detile，属正常现象，别关** |
| 流送完成 | 稳定 21–26 fps | 可游玩状态 |

## 关键参数与实测教训

本配置经三轮实测（run2 崩溃 / run3 卡顿 / run5 定稿）敲定：

### 防崩溃（run2 教训：静默死亡，无任何错误日志）

| 参数 | 值 | 原因 |
|---|---|---|
| `VICTORY_TEXTURE_BUDGET_MB` | `6144` | 源码注释原文：*"Past the budget the system moves the card's memory into the PC's, which ended runs 20 and 24"* —— 显存超预算会被 Windows 强制挪进内存，进程直接死 |
| `VICTORY_TEXTURE_ROOM_PERCENT` | `80` | 给系统留 20% 显存余量（默认 94% 在 12GB 卡上必撞墙） |
| `VICTORY_TEXTURE_MIP_DROP` | `1` | 纹理降 mip，省约 75% 纹理显存 |

### 防卡顿（run3 教训：4–6 fps，GPU 时间暴涨 30 倍）

| 参数 | 值 | 原因 |
|---|---|---|
| `VICTORY_CARD_BUFFERS_MB` | `1024` | **切勿降到 512**——分块减半导致 GPU 复制碎片化，GPU 时间 150 ms/帧（正常 <5 ms），copies 流量翻倍 |
| `VICTORY_FRAME_INTERVAL_MS` | `33.3` | 30 fps 档。CPU 翻译线程实际 25–40 ms/帧，60 fps 档追不上反而积压 |

### 其余约 60 个开关

为 fork 作者 3070 基线脚本的直译，逐项说明见 `配置文档.md` 第 3.4 节。

## 排错速查

| 症状 | 根因 | 处理 |
|---|---|---|
| 静默崩溃（日志无 ERROR、无 WER 事件） | 显存超预算被系统驱逐 | 确认 `TEXTURE_BUDGET_MB=6144` 与 `ROOM_PERCENT=80` 存在 |
| 持续 4–6 fps，GPU >100 ms/帧 | `CARD_BUFFERS_MB` 被改小 | 回到 `1024` |
| 持续卡顿但 GPU 时间 0 ms | CPU 翻译瓶颈 | 关诊断统计（`TABLE_MEMO`/`QUEUE_MARKS`/`LIST_ALIAS_EXACT`）或 `MAP_THREADS=2` |
| Windows 事件查看器出现 `RADAR_PRE_LEAK_64` | 内存泄漏前兆 | 同防崩溃处理，减纹理预算 |

## 键盘控制

```text
WASD → 左摇杆      IJKL → 右摇杆     Dpad → 方向键
A → 回车键         B → 删除键        X → X        Y → Y
左 trigger → QE    右 trigger → ZC
左摇杆按下 → F     右摇杆按下 → H
主菜单 → ESC       视角 → TAB
```

## 源码

本包对应的完整构建源码保留在 git 历史中：

```bash
git checkout 6645043   # victory-scarlett-3070 完整源码树
cargo build --release --bin victory --locked
```

## 免责声明

本项目仅用于**个人已合法拥有的游戏数据的互操作性研究与技术学习**，不分发、不包含任何游戏资产（ROM / 资源文件 / 密钥）。所有商标归各自权利人所有。
