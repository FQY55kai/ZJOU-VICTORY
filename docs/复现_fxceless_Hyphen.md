# 复现 fxceless/victory-gta6（Hyphen）结果报告

**时间**：2026-11-25 03:47 – 04:10
**机器**：机械师曙光 16 Pro / i9-13900HX / RTX 4080 Laptop / Windows 11
**目标**：复现 gta6recomp.com 排行第 2 的开源项目 Hyphen（fxceless/victory-gta6，78%）

---

## 一、结论

**复现成功。** 我方机器上：

| 阶段 | 结果 | 证据 |
|---|---|---|
| 构建 | 成功（release，30 秒增量） | `D:\victory-fx\target\release\victory.exe`（7.5 MB） |
| 启动 / 加载插画 | **完整渲染** | 窗口抓屏 lit=43.9%、彩色=33.7%，插画（持枪人物 + 迈阿密街景）完整可见 |
| 进入游戏内 | **渲染 3D 场景** | 6569 draws/frame、窗口 lit=4.46%、可见室内几何 |
| 帧率（默认） | 0.2–0.3 fps（3550–4640 ms/帧） | 与 fxceless README「3–6 秒/帧」一致 |
| 帧率（推荐开关） | **2.6 fps（385.8 ms/帧）** | 与 fxceless 声称「2–4 fps」一致 |
| Shader 翻译失败 | **0 条** | 对照我方 JMAN 树 21 条失败 |

**推荐开关**（fxceless README 原话，实测有效）：
```
VICTORY_RELAY=2 VICTORY_LATE_WAIT=3 VICTORY_WIDE_BACK=1 VICTORY_CARD_BUFFERS_MIN_MB=1 VICTORY_CARD_BUFFERS_MB=1024
```
提速比：3550 ms/帧 → 385.8 ms/帧，约 **9 倍**。

---

## 二、关键情报（来自 gta6recomp.com 排行榜）

| 排名 | 项目 | 进度 | 开源 | 状态 |
|---|---|---|---|---|
| 1 | omonkey | 79% | ❌ 私有 | 监狱场景 5 FPS，Rumble 直播，无源码 |
| 2 | **Hyphen（fxceless/victory-gta6）** | **78%** | ✅ **开源** | 3 FPS @ RTX 4090，监狱外景 + 光照 + 教学提示 |
| 3 | CGBJ | 72% | ❌ 私有 | 序章过场、控制器行走 |
| 4 | VIctory（Marvisak 上游） | 70% | ✅ | In-Game 黑屏，30 FPS 菜单 |
| 5 | AnyXboxX（LooperSalty） | 65% | ✅ | 角色可控 + 教学提示，3.2 FPS |
| 6 | XCL | 63% | ✅ | 世界加载、行人车辆生成 |
| 7 | Xbox2Windows | 48% | ✅ | 静态重编译，首帧呈现 |
| 8 | Palmetto | 25% | ✅ | 可执行映射、TLS 追踪 |

**最重要的一条**：排行榜明确写「Hyphen's patches are upstreamed」——**fxceless 的补丁正在合入 Marvisak/VIctory 上游**。这意味着我们的正确路线是跟踪 `Marvisak/VIctory` main（而非仅 JMAN 分支），JMAN 只是中间 fork。

---

## 三、fxceless 相对我方 JMAN 树的核心差异

| 项 | fxceless | 我方（JMAN 50bda73+9） |
|---|---|---|
| Base | upstream main `ffe310e`（706+ PR 合入） | JMAN fork（322 提交，base 更旧） |
| 矩形列表（我们黑屏的直接病灶） | **着色器注入几何阶段 `rect_list_gs`，GPU 补第 4 顶点**；CPU 展开仅兜底；整删 `rectangles.rs`（-956 行） | CPU 展开（`primitive.rs::expand_rects`）+ 独立 compute 展开器 |
| Wave64 | `e9765a7` 修复设备挂死：barrier-in-branch 形式默认关，`VICTORY_WAVE_WIDE=1` 才开 | 无 `VICTORY_WAVE_WIDE` 机制；shader 直接声明 `WaveSize(64)` 被驱动拒绝 |
| Shader 缓存 key | 含 `VICTORY_WAVE_WIDE`/`WAVE_DEEP_SKIPS`/`WAVE_OWN_BITS`/`F16_NEAREST` | 不含这些开关 |
| 翻译失败 | 0 | 21（compute 18 / mesh 3） |
| 内存/速度 | `VICTORY_LATE_WAIT`/`VICTORY_RELAY`/`VICTORY_INPLACE`/卡内存缓冲 | 无 |

---

## 四、我方当前卡点（与 fxceless 对照后的判断）

我方黑屏已定位到**最终合成 pass**：

- UI 渲染进 RT `0x814ec56c90`（3840×2160，lit 99.9%）✓
- 最终合成 = **topology 17 = RECTANGLES** 单矩形全屏 draw，目标为交换链图像
- 该 draw **执行了**（`ran`），矩形顶点数据**完全正确**：
  `(-1,1,0) (-1,-1,0) (1,-1,0)` + UV `(0,0)(0,1)(1,1)`，颜色为 Unorm8
- 但交换链图像仍全黑（lit 0%）→ 合成结果没写进去

fxceless 的做法绕开了我方 CPU 展开路径，直接由着色器几何阶段完成矩形补点——**这正是"矩形列表已修复"所指**。

---

## 五、目录与命令

| 用途 | 路径 |
|---|---|
| fxceless 工作树 | `D:\victory-fx`（git worktree，分支 detached @ 7b0f736） |
| release 二进制 | `D:\victory-fx\target\release\victory.exe` |
| 运行日志（默认） | `D:\victory-fx\_debug_logs.log` |
| 运行日志（提速开关） | `D:\victory-fx\_debug_logs2.log` |
| 我方 JMAN 树 | `D:\victory-ref`（HEAD 含 9 个新 JMAN 提交） |
| 我方 diag 二进制 | `D:\victory-ref\target\diag\victory.exe`（**必须** `--profile diag --features diagnostics` 构建） |

复现命令：
```bash
cd /d/victory-fx && VICTORY_RELAY=2 VICTORY_LATE_WAIT=3 VICTORY_WIDE_BACK=1 \
  VICTORY_CARD_BUFFERS_MIN_MB=1 VICTORY_CARD_BUFFERS_MB=1024 \
  ./target/release/victory.exe run "C:\boottest\game_scarlett_master.exe" \
  --execute --unimplemented zero --console series-x
```

**注意事项**：
1. `cargo build ... | tail -1` 会吞掉失败状态（管道退出码是 tail 的），必须显式检查 `$?`。
2. 构建前必须确认没有 `victory.exe` 在跑，否则替换 exe 报「拒绝访问 (os error 5)」。
3. 只开一个实例（两个 exe 同时跑会内存溢出）。
4. `grab_window.py` 只匹配标题含 `Americas` 的窗口（原逻辑会把浏览器里的 VIctory 页面抓进来）。
