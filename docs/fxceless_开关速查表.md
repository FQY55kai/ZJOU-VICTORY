# fxceless/victory-gta6 可调开关速查表

来源：`D:\victory-fx` 全仓 grep `env::var("VICTORY_*")`，共 41 个。以下按用途分组，**已实测有效的标 ★**。

## 一、提速（README 推荐组合，实测 3550 ms/帧 → 385 ms/帧）

| 开关 | 作用 |
|---|---|
| ★ `VICTORY_RELAY=2` | 提交提前送出 + relay 线程在显卡完成时落结果；README 首选 |
| ★ `VICTORY_LATE_WAIT=3` | 提交越过等待（3 = 组合档） |
| ★ `VICTORY_WIDE_BACK=1` | 宽回写路径 |
| `VICTORY_LATE_WAIT_LISTS` | 同上，按命令列表粒度 |
| `VICTORY_PIPELINE_WAIT_MS` | 主机管线等待超时（默认见代码） |
| `VICTORY_UNWATCHED_FRAMES` | 内存监视间隔，1 = 每帧（越小越准越慢） |
| `VICTORY_CARD_WINDOWS_MB` | 卡内存窗口大小 |

完整启动命令（当前在跑的组合）：
```bash
VICTORY_RELAY=2 VICTORY_LATE_WAIT=3 VICTORY_WIDE_BACK=1 \
VICTORY_CARD_BUFFERS_MIN_MB=1 VICTORY_CARD_BUFFERS_MB=1024 \
./target/release/victory.exe run "C:\boottest\game_scarlett_master.exe" \
  --execute --unimplemented zero --console series-x
```

## 二、内存（当前约 24 GB 占用，是掉帧主因之一）

| 开关 | 作用 |
|---|---|
| `VICTORY_TEXTURE_BUDGET_MB` | 纹理副本的字节上限 |
| `VICTORY_KEEP_SNAPSHOTS_MB` | 快照保留预算（默认 256，历史上是 768） |
| `VICTORY_ROOT_UAV_MB` / `VICTORY_ROOT_VIEW_MB` | 根 UAV / 根视图缓冲预算 |
| `VICTORY_INPLACE=1` + `VICTORY_INPLACE_MB` | 标题内存映射成图形堆，缓冲原地用、不复制 |
| `VICTORY_INPLACE_PROBE` / `_PROBE_MB` | 原地方案的探针与统计 |

## 三、着色器 / 管线（影响画面正确性）

| 开关 | 作用 |
|---|---|
| `VICTORY_GROUP_SYNC=0` | 关掉组屏障等待（换速度，可能出错） |
| `VICTORY_HOST_WAVE=32` | 按 32 lane 主机的波宽翻译（我方 warp32 场景强相关） |
| `VICTORY_VIEW_LIMIT=<槽位>` | 按视图堆受限的主机翻译（NVIDIA 上限 1,000,000） |
| `VICTORY_SKIP_PIPELINES=<hash,...>` | 跳过指定管线（隔离问题用） |
| `VICTORY_STACKS` | 每线程栈大小（光追遍历深度） |

## 四、诊断

| 开关 | 作用 |
|---|---|
| `VICTORY_LOG=模块=级别` | 日志过滤，如 `victory_gpu::draw=debug,victory_gpu=info` |
| `VICTORY_CAPTURE_FRAMES=256,512` | 指定帧抓图 / 开每-draw 调试行 |
| `VICTORY_DUMP_MATCH` | 匹配转储 |
| `VICTORY_SAVE_OP` / `VICTORY_SAVE_STOP` | 保存失败操作现场 |
| `VICTORY_CRASH_SAVE` | 崩溃时保存状态 |
| `VICTORY_EMIT_DUMP` | 转储翻译后的中间表示 |
| `VICTORY_BARRIER_TEST` / `VICTORY_SCISSOR_TEST` / `VICTORY_MESH_TEST` | 定向自检 |
| `VICTORY_WATCH` / `VICTORY_PAIRS` | 内存监视与成对跟踪 |
| `VICTORY_KEYS` | 按键调试 |
| `VICTORY_DISPLAY_MODE` | 显示模式覆盖 |
| `VICTORY_WARP=1` | 软件适配器（WARP）跑，用于对照 |
| `VICTORY_XMA_REFERENCE` / `VICTORY_XMA_STREAMS` | 音频 XMA 参考实现 |

> 注意：README 里的 `VICTORY_CARD_BUFFERS_MB` / `_MIN_MB` 在代码中并不存在（代码里是 `VICTORY_CARD_WINDOWS_MB`），属于**旧名残留**，传了也无害（被忽略）。

---

## 我方机器实测状态（2026-11-25 04:20）

- 帧率：默认 0.2–0.3 fps → 加开关 **2.5–2.6 fps**；用户实机 3–5 fps（4 fps / 241 ms 见于标题栏）
- **16 个 shader 翻译失败**（fxceless 在 RTX 4070 上是 0）；**314 draws/帧被永久丢弃**（0 条管线在建）
- 失败类型分布：
  | 次数 | 错误 |
  |---|---|
  | 6 | `a descriptor read only as data is not supported` |
  | 2 | `v_cvt_f32_f64 is not supported` |
  | 2 | `the first half of an interpolation is not supported` |
  | 2 | `a wave reduction where not every lane runs is not supported` |
  | 2 | `a lane mask holding lanes elsewhere read as data` / `a lane mask exchanged where not every lane runs` |
  | 1 | `v_permlanex16_b32 is not supported` |
  | 1 | `a descriptor's address or format read as data is not supported` |
- 已确认 JMAN 分支里有对应修复但**未合入 fxceless**：`e498078`（interpolation halves）、`39979ac`（v_permlanex16_b32）。两者与 fxceless 的 `io.rs` / `lower.rs` 有结构性冲突，手工合并约 300 行。
- 光照缺失是**文档化缺口**：`docs/fork-ray-tracing.md` 开篇即写 “nothing is lit by ray tracing yet”，太阳直射之外全黑即为预期表现，**没有任何开关能打开**。
