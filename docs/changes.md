# 自研改动清单与代码结构

相对全部上游的改动，以及各部分的代码位置。

### 五、自研改动清单（相对全部上游）

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
| `f2b8c85` | **pixel shader 的 quad 读取走 quad read（含 helper lane）**：修三角形边线（车漆/玻璃/皮肤），移植自 Belobug `03629f3` |
| `1fc58f3` | **常量间接漫反射光**（`VICTORY_AMBIENT`，默认 0.6）：光追不跑时替代 resolve 输出，修室内色块/洋红植被/黑人脸，移植自 Belobug `4f45ab8` |
| `c7be77e` | **鼠标映射**：移动转镜头（右摇杆，自动衰减）、左键开火、右键瞄准、中键 A |
| `6ad4317f` | **合并 LooperSalty `80d58f8` 27 提交**（AnyXboxX 钉住的同一批）：DirectStorage 物理页写通知（`VICTORY_DSTORAGE_NOTIFY`，修囚室墙透明）、xmem 按 section 索引、sampler 比较槽、MIMG 偏移、BCPack 词典、COPY 队列卡上取回、depth bounds 直通宿主管线、离线预编译（只留 `victory warm`）。19 处冲突全清，330+ 测试通过；**帧 346 → 172 ms、GPU 220 → 76 ms** |
| `edf97609` | **多树合并后的去重**：删掉与 `descriptor_words` 重复的 `descriptor_data` 整份（276 行）及 `FIRST_HALF`、`pages_written`、`shown_at`、被 `recheck_looks` 取代的 `recheck` |
| `477c395a` | `placed_filler_twins` 按 `feature="diagnostics"` 恢复（诊断内存行要读它） |

---

### 九、代码结构（分门别类）

12 万行、142 个源文件。按职责分五个 crate，每帧热路径只在 `victory-gpu` 与 `victory-shader`：

| crate | 行数 | 职责 | 热路径 |
|---|---|---|---|
| `victory-shader` | 43,381 | xbox 着色器字节码 → DXIL 翻译器（IR、结构化、发射、验证） | 首次遇到时（缓存后为 0） |
| `victory-gpu` | 44,365 | D3D12 后端：清单提交、title 内存镜像、描述符堆、纹理流送、卡上哈希 | **每帧** |
| `victory-hle` | 19,451 | 内核/系统模块替身（文件、内存、DirectStorage、输入、音频） | 每帧（浅） |
| `victory-formats` | 4,536 | 容器格式（BCPack 词典等） | 载入时 |
| `victory-loader` / `victory-pe` / `victory-game` / `victory-core` / `victory-pipeline` | 7,141 | PE 加载、重定位、游戏特定补丁、管线库 | 启动时 |

**`victory-gpu` 内部按职责分文件**（括号内为行数）：

| 文件 | 职责 | 对应账本环节 |
|---|---|---|
| `host.rs` (6,954) | 提交调度、飞行队列、卡上工作、回读中继、统计行 | 2/9/10 |
| `resource.rs` (4,058) | 堆与资源放置、孪生拷贝 | 4 |
| `shadow.rs` (3,752) | title 内存镜像、着色器写检测、变更段合并 | 4 |
| `descriptor/host.rs` (3,570) | 描述符堆增量、槽位与视图 | 5 |
| `draw.rs` (3,531) | 清单录制、绘制/间接分派、满显存重传 | 7 |
| `list.rs` (2,222) | 清单结构与值等待 | 7/10 |
| `indirect.rs` (1,931) | 间接参数解析与拆分 | 7 |
| `texture.rs` (1,681) | 纹理变化检测与解瓦 | 3 |
| `queue.rs` (1,086) | 命令队列、present、帧率行 | 1/6 |
| `ambient.rs` (242) | 常量间接光（`VICTORY_AMBIENT`），RT GI 的占位 | 8 |

**`victory-shader` 的翻译器**（本次拆分，原先全在 `emit/lower.rs` 一个文件里）：

| 文件 | 行数 | 职责 |
|---|---|---|
| `emit/lower.rs` | 10,570 | 降级本体：一条指令一条指令地发射 DXIL |
| `emit/lower/waves.rs` | 757 | lane/wave 分析：值需要哪些 lane、mask 被当数据读、哪些程序能按 wave 跑 |
| `emit/lower/pack.rs` | 172 | 签名打包：半精度通道落在宿主参数的第几个 dword |
| `emit/lower/tests.rs` | 659 | 上述三者的测试 |

拆分前 `lower.rs` 是 **12,133 行 / 526,401 字节**，超过仓库自定的 512 KiB 上限（`xtask/src/policy.rs`），策略测试一直是红的。拆完 **10,570 行 / 461,934 字节**，策略测试首次通过。拆出的三个文件已加入缓存 key 的源列表（`lib.rs::sources()`），因此**这一改动会让着色器缓存失效一次**。

#### 本次顺带修掉的一处开销

`shadow.rs` 的变更段原先只切“精确相异的字节”：中间哪怕只有 1 个相同字节也另起一段。实测 run 6 每帧 **56,440 段 / 0.15 MB（平均 2.8 字节一段）**，每段都要一次拷贝、一条发给卡的范围、一条 dirty 记录。现在 `changed_runs_within` 把 **≤16 个相同字节的间隔并入同一段**（拷贝相同字节等于写入它原本的值，无害）：

| 指标 | 改动前（run 6） | 改动后（run 7） |
|---|---|---|
| 单次合并的段数（中位） | **23,797**（峰值 114,553） | **9**（峰值 43） |
| 每段承载的字节 | 2.8 B | 约 16 KB |
| 触发合并的采样占比 | 148 / 345 | 6 / 288 |
| 稳态帧 | 4.6–6.6 fps / 151–239 ms | 5.0–5.3 fps / 190–200 ms |

**帧率没有可测变化**——这一项本来只占帧时 1–3%，如实记录，不吹。真正省掉的是每帧数万次的小拷贝与 dirty 记录，以及由此引发的反复 settle。

---

