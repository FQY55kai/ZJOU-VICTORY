# 上游同步记录

逐条合入、冲突、以及与上游的技术分歧。**这是流水账，不是结论**；结论在 README。

### 十、上游同步（2026-11-25 夜）

#### 榜单已经变了

| 项目 | 进度 | 变化 |
|---|---|---|
| **DexNom/VIctory** | **78%** | 新登顶；**首个 AMD 跑通光追光照**，说明为"接近零售版"，3–4 fps，很少崩 |
| Belobug/VIctory-Belobug-AMD | 77.4% | 我们已移植过它的 quad read 与 AMBIENT |
| Zettrox/ztrx | 64.2% | 打通序章到 Jason 首任务"Gone Fishing" |
| fxceless/victory-fork | 59.8% | 我们另一个参考源 |
| Marvisak/VIctory（上游） | 40% | 我们全部代码的祖宗 |

#### 上游领先我们多少

分叉点 `ffe310eb`（11-24 10:53）之后，**上游主线 319 个提交、其中 182 个非合并提交**，我们 106 个、117 个文件、+53k 行。**直接 `git merge` 产生 411 处冲突 / 55 个文件**：

| 文件 | 冲突块 | 文件 | 冲突块 |
|---|---|---|---|
| `emit/lower.rs` | 65 | `emit/shader.rs` | 17 |
| `draw.rs` | 38 | `shadow.rs` | 15 |
| `host.rs` | 35 | `emit/mod.rs` | 13 |
| `descriptor/host.rs` | 19 | `texture.rs` | 13 |

冲突这么密，是因为上游在 **11-24 22:36 有一整块大型功能落地**（title 内存按裸地址走页表 `27c3dc9d`、FidelityFX 与 64 位图像原子 `0b35b7fe`、BVH 求交、shader pack `286e33f0`、根视图走页表 `0f637570`）——它重写了我们改动最深的 `shadow.rs` / `host.rs` / `memory.rs`。**盲合会把双方都毁掉**，所以按“逐个提交审查后挑取”来做。

#### 本轮合入（逐条人工核对冲突）

| 上游提交 | 内容 | 我们的处理 |
|---|---|---|
| `4e60ea3a` | 无宿主管线时**先丢间接调用，再做参数刷新与钳位 dispatch**；上游实测每帧 1294–1750 个这类空转 | 我们 `execute_indirect` 早已把 `host_pipeline` 查表提到最前并绑定 `p`（矩形与类型判断都要用），比它更彻底 → **保留我们的结构，只采纳它的诊断原因 `why("no host pipeline")`** |
| `06019553` | **helper lane 的 `v_readfirstlane` 读自己的值**：否则 waterfall 循环里只剩 helper lane 时会空转到 `LOOP_LIMIT`，单次绘制 0.3–1.3 秒，约 200 秒后 `DEVICE_HUNG` | 我们的 `v_readfirstlane_b32` 走双 host-wave 交换（`first_lane` + `exchange_halves`），该结构保留；在其后追加它的 `helper_reads_own` 选择，helper lane 取自己的值 |

验证：全量 **563 个测试通过、0 失败**，`cargo xtask policy` 通过，诊断构建上机运行**零 `DEVICE_HUNG` / 零设备丢失**。

#### 有意没合入的（附阻塞原因）

| 上游提交 | 价值 | 为什么这轮不合 |
|---|---|---|
| `cd94d353` + `940d32f6` | shader 标记它们经视图写过的页，回读只比那些页——正打我们 42.7 ms 的 copy back 与 234 遍卡上哈希 | 牵动 **20 个文件**，依赖上面那一大块上游落地；单挑必然造出一棵半新半旧的树 |
| `ed0dbfd1` | shader 声明范围内若有 title 写入，副本会保持陈旧（某 RTX 4050 上 dispatch 读到 0 尺寸死循环直到设备移除） | 依赖上游 shadow 重构（`tracked` 字段、`before` 结构都不同）。**我们树里确实有同一缺陷**（`Shadow::update` 整段跳过 `pending` 范围），但修法要同时改快照，否则回写会覆盖 title 的后续写入——写回路径最怕内存损坏，不硬凑 |
| DexNom `bc3cd4b0` 等 6 个 RT 提交 | **光追光照本身** | 依赖 `cd94d353` 与那一大块落地；它的 `memory.rs +396`、新增 `writeback.rs 321` 正是建立在我们没有的地基上 |

另外两条原计划要搬的改动，查证后**已被上游自己的机制取代**，不再移植：

| 我们的改动 | 上游的替代 |
|---|---|
| `shadow.rs` 的 `changed_runs_within`（变更段合并） | 上游 `cd94d353` 改成**页标记**：shader 声明它经视图写过哪些页，回读只比那些页。逐字节差分这整条路径在上游已不存在（`git grep "fn changed_runs" 940d32f6` 为空） |
| `descriptor_data` 去重 | 上游的页标记落地后重复已不存在（`fn descriptor_data` 与 `fn descriptor_words` 在 `940d32f6` 中各只出现 1 处） |

#### 更正：光追的着色器层早就在我们树里（2026-11-26 凌晨）

上一版写的"最后叠 DexNom 的 RT 提交"这个计划，查证后发现前提是错的：

| 查证项 | 结果 |
|---|---|
| DexNom/fxceless 的光追着色器提交 `6a2d9f99`（"Shadow and trace programs translate"） | **`git merge-base --is-ancestor 6a2d9f99 0b969193` = 是。它已经是我们的祖先**，再 cherry-pick 是空操作 |
| 该提交新增的文件 | `own_bits.rs`（914 行）、`wave64_tests.rs`（452 行）、`docs/fork-ray-tracing.md`——**三者在主树里都在**，`emit/mod.rs` 也已 `mod own_bits; mod wave64_tests;` 接线 |
| `image_bvh*_intersect_ray` 的模拟 | `crates/victory-shader/src/emit/intersect.rs`（557 行）+ `src/bvh.rs`，来自同一支线 `578d459c`，`lower.rs` 已 `#[path = "intersect.rs"] mod intersect;` |
| `lower.rs` 的接入点 | 已接：`own_bits: super::own_bits::OwnBits`（273 行）、`own_bits::find(...)`（634 行）、循环里 `self.own_bits.alike` / `totals`（6773、880 行） |

也就是说：**我们缺的从来不是光追的着色器代码，那部分早在树里了。**

#### 上游走的是另一条路（同一条指令，两种实现）

上游 `940d32f6` 里也有一条 `image_bvh_intersect_ray` 的实现，但和我们这支完全独立：

| | 我们（fxceless / ZSlayer / DexNom 支线） | 上游 `940d32f6` |
|---|---|---|
| 求交模拟 | `emit/intersect.rs` 557 行 + `src/bvh.rs` 696 行 | `emit/lower/bvh.rs` 535 行（`0b35b7fe`） |
| 波前 64 的处理 | **车道模拟**：`own_bits`、`structure_wide`、`without_early_outs`、`without_skips`、`widen`、`with_local_waves`——把 console wave64 拆成两个 host wave 再用 LDS 交换 | **直接声明**：`lower.rs:541 s.wave_size = Some(wave)`，在 DXIL 里要 `WaveSize=64`，让设备自己支持 |
| 代价 | 约 2,300 行模拟代码，翻译慢、逻辑绕 | 设备不支持 wave64 就翻译不出来（NVIDIA 的 D3D12 `WaveLaneCountMax` = 32） |

上游那 6 个测试在这台机器上本来就失败（`permutes_read_the_lane_named_and_zero_from_lanes_left_out` 等），正是这条分歧的副作用。

#### DexNom 的 `570a941e` 是一个假补丁

`570a941e`（"Publish DexNom Runtime Radeon sources"，170 文件 / +49,715 −36,762）的父提交**正是** `940d32f6`，但它的树**不是**"上游 + 光追"，而是**整棵 fork 树的快照**。证据是它相对上游删掉了上游独有的模块：

| 被删掉的上游模块 | 用途 |
|---|---|
| `bindless.rs` | bindless 纹理队列 |
| `external.rs` / `switch.rs` / `tdr.rs` / `twin.rs` | 外部资源、开关、TDR、孪生拷贝 |
| `emit/lower/bvh.rs` | 上游自己的 BVH 求交 |

**结论：把 `570a941e` 合进来等于用 fork 树覆盖上游 319 个提交的成果**，冲突只有 5 文件 / 9 块（正好是我们刚移植过的那 5 个文件），但合完就退回上游之前。这条路不走。

#### 真正的缺口：一道硬门

光追在本树里的最后一步不是写代码，是 `crates/victory-gpu/src/pipeline.rs` 里这道门：

```rust
// shortcut: and only with `VICTORY_BVH_BUILDER=1`, until a run has shown
// the whole builder and the host side of title memory by address
// (`memory.rs`) working on a card; nothing reads the structures before
// the programs that walk them translate.
fn builder_runs() -> bool {
    if !ALLOWED.get_or_init(|| env::var_os("VICTORY_BVH_BUILDER") == Some("1".into())) { return false; }
    ...
}
```

门上的注释写着解锁条件："**直到遍历结构的程序能翻译**"。而按上面查证，遍历程序（`intersect.rs` + `own_bits.rs` + `wave64_tests.rs`）**确实能翻译**——所以门的前提看上去满足了，才值得实际开一次。**实测结果是：遍历程序能翻，构建器还差 3 个**（见下一节）。

配套的诊断开关也在，正是为首次开闸准备的：

| 开关 | 作用 |
|---|---|
| `VICTORY_BVH_QUARANTINE=1` | 构建器写的东西只留在宿主副本、不回写 title 内存——首次跑全构建器的保险（"a stage that stores where it should not can then not reach the title's own objects"） |
| `VICTORY_RT_RESOLVE=1` | 让 resolve 管线被造出来 |
| `VICTORY_WAVE_WHY=1` | 打印 wave64 重试的每一步（`own_bits` 走到哪条路） |

**开闸的代价（必须预备好）**：`pipeline/make.rs::without_structures` 在构建器关着时会把 `CS_RaytracingResolve` 整批丢掉——注释写明那是**约 1,000 个计算程序，全部绑定在世界第一帧，每个要 AMD 编译器 5–9 秒**。所以：

- `VICTORY_BVH_BUILDER=1` 一开，这 1,000 个程序全部要造出来，**首次进世界的光编译时间约 1.4–2.5 小时**（5–9 s × 1000）。
- 这不是 bug，是"构建器关着就没有命中可着色"的必然结果，也正是它默认关着的原因。
- 缓解：着色器缓存（`hits=1846 misses=0` 的那套）会把这 1,000 个的翻译结果留下来，**第二次启动起为 0**。所以开闸前务必确认缓存目录可写、且 `lib.rs::sources()` 不再变动（任何源文件增删都会让缓存整体失效一次）。

#### 本轮实际做完的事

在独立 worktree `D:/victory-up`（分支 `up-base`，基 `940d32f6`）里，按 crate 逐块搬我们的改动，每块跑测试：

| 提交 | 内容 | 验证 |
|---|---|---|
| `75afad29` | 鼠标映射（`gameinput.rs`：`scripted_keys()` 与 `mouse()` 并存，`merge_scripted` 走 `fallback(scripted, MouseState::default())`） | 编译通过 |
| `1691f839` | quad-local read 到达 helper lane，收在 `VICTORY_QUAD_HELPERS=1` 后面（上游的规则是它自己硬件测试写的那条，而这台机器上那 6 个测试本来就是红的，无法判定对错，所以默认保持上游行为） | 编译通过 |
| `fc472c36` | 常量间接光 `AMBIENT`（`ambient.rs` + `pipeline.rs` 的 `resolves_light` + `draw.rs` 的 `self.ambient(list, p)`） | `victory-gpu` **144 passed / 0 failed** |

`relay/flight`（`VICTORY_RELAY=1` 独立回读线程）在这棵树上**没有对应物**——上游有自己的多 flight 实现，要做得端到端重做，不在本轮范围。

#### 开闸实测（2026-11-26 00:15–00:25）：53 个程序里 3 个翻译不出，而这道墙在这张卡上过不去

先加了一个开关把两件事拆开：`VICTORY_BVH_RESOLVE=0` 只跑构建器、不造那 1,000 个 resolve 程序。**验证时间从预估的 1.4–2.5 小时压到 3 分钟**（`pipeline/make.rs::without_structures`）。

`VICTORY_BVH_BUILDER=1 VICTORY_BVH_QUARANTINE=1 VICTORY_BVH_RESOLVE=0 VICTORY_WAVE_WHY=1` 跑两次（`rt1`、`rt2`）：

```
host wave lanes min=32 max=32
the ray-tracing structure builder stays off: not all of its programs have a host pipeline,
and its stages run all or none    missing=3 of=53
```

**构建器实为 53 个程序**（文档写 52），其中 3 个翻译不出，all-or-none 规则让整批关闭：

| 着色器 | 规模 | 报错 |
|---|---|---|
| `b22a16a1` | 28–56 次/帧 | `0x22c: a wave sum of 16-bit floats where not every lane runs` |
| `f133e35e` | 69,384 字节，0.2–2 次/帧 | `0x1eb4: a lane mask exchanged where not every lane runs` |
| `ceb5eb68` | 早期一次 | `0x38: a descriptor's address or format read as data` |

重试机制确实在跑（944 条 `WAVE64-*` 记录）：`without early-outs`、`without skips`、`without skips, scalars followed`、`local waves (idiom)`、`local waves (APPROXIMATE)` 全部试过，都停在同一处。

**`VICTORY_WAVE_WIDE=1` 打开 `widen()` 后**：救回 **33 个**程序（返回 `wave Some(32)`，即不再要求 64 车道），但构建器那 3 个仍然不行——`widen` 被拒 14 次，理由统一是：

```
WAVE64-WIDE not tried: thread groups of [256, 1, 1]
WAVE64-RETRY wave-wide branches: wave-wide branches in a thread group of several console waves
```

而且这次运行在进世界约 2.5 分钟后**整机静默**（进程活着、占用 15.8 GB），同样时长下不开 `VICTORY_WAVE_WIDE` 的 `rt1` 没有这个现象——与 `mod.rs:404` 那段注释（"runs 121–124 lost the card … `ptfx_CS_lightAtlas_Sprite` 和 `_Trail`"）完全一致。**默认关是对的，不要再默认打开。**

##### 为什么这是结构性的，不是缺一行代码

console 的一个波是 64 车道。卡上只有 32 车道时，跨 64 车道的运算（f16 平衡树求和、lane mask 交换）必须让两个 32 车道 host wave 通过 LDS 交换，而交换需要**组屏障**。组屏障一旦落在发散分支里，没到齐的线程就永远等下去——所以 `widen()` 只在"线程组正好是一个 console 波（64 线程）"时才敢用，这是它 `cs.threads.product() == 64` 那道判断的全部理由。

构建器的线程组是 **256 / 128 / 1024**，天生跨多个波。三个失败程序要的恰恰是"每个车道都到场"，而这在多波组里做不到安全。

| | AMD（`WaveLaneCountMax=64`，RX 7900 XTX） | NVIDIA（`min=32 max=32`，本机 RTX 4080） |
|---|---|---|
| 上游路线：声明 `WaveSize=64` | ✅ 可用，根本不需要模拟车道 | ❌ D3D12 不支持，`min=max=32` |
| 我们的路线：模拟车道 | ✅ 也不必模拟 | ❌ 跨波交换需要组屏障，多波组里死锁 |
| 构建器 53 个程序 | 应可全翻译 | **3 个永远翻译不出** |

**这才是榜单上 DexNom 78% / Belobug 77.4% 的物理前提**——他们都在 AMD 上，`WaveSize=64` 直接可用。换到 wave32 上限的卡，光追这条路在当前设计下是关着的，跟努力程度无关。

##### 剩下两条路

1. **换一张 AMD 卡验一次**（最省事，1 小时见分晓）：同一份代码在 `min=32 max=64` 的卡上，`widen` 与 `local waves` 都会走别的分支，构建器很可能直接起来。这是把"3 个失败"变成"0 个"最快的验证方式。
2. **留在 NVIDIA 就要改发散模型**：把 `structure_wide` 从"按波"扩到"按组"——让组里所有波对每个分支做同一个决定（谓词化，谁都不提前退出），屏障才合法。这等于重写结构化器的一部分，工作量大，且拿不到正确 BVH 时的后果已知（早期尝试是"第一次构建器提交后一秒内死在框位模式指针上"）。不建议在没拿到 AMD 数据前动。

##### 本轮附带产出

| 改动 | 作用 |
|---|---|
| `VICTORY_BVH_RESOLVE=0` | 只跑构建器、不造 1,000 个 resolve 程序；把首次开闸的验证从小时级压到 3 分钟 |
| 实测数据 | 构建器 53 个、失败 3 个及其 hash 与偏移；`widen` 救回 33 个、被拒 14 次的原因 |

#### 2026-11-26 00:30 再看上游

| 远程 | 头部 | 内容 |
|---|---|---|
| `origin/main`（Marvisak） | `67e8ae14` | `940d32f6` 之后又 **85 个提交**，绝大多数是 GPU 性能重构：页标记回读、纹理暂存池、单视图共享大拷贝缓冲、堆同步只处理变动槽位、D3D12 对象专用释放线程、快照池上限 512 MiB |
| `dexnom/main` | `921d8d3b` | `Repair pixel quad sampling, grow ray buffer views, and recover sparse textures` —— 光追方向继续前进，仍在他们那棵树上 |
| `looper/perf/nt-write-copy-back` | `6950319c` | copy back 1.5 s → 0.13 s（`NtWriteVirtualMemory`，不先 `VirtualQuery`）。**我们树里已有**，见第十一章 |
| `looper/perf/copy-pool-stacked` | `8cce7801` | 拷贝线程池做快照与比对。依赖上游的页快照，拿不到 |
| `looper/shaders/*` | `b00573d8` / `8996aeb8` | `v_readfirstlane` 地址下的 code-table 与 loop counter，属 waterfall 方向 |

**两条直接关系到我们待办的上游提交**：

- `4015a182 Shaders: quad reads in pixel programs reach the helpers at triangle edges` —— **上游自己实现了我们的 `VICTORY_QUAD_HELPERS`**。本树可以改用它，不必再挂环境变量。
- `e8e9f0c2 Shaders: a lane mask joined with data at a uniform join is its number` 与 `4a3166e6 Shaders: lanes skipping a compare's region read 0 of its lane mask` —— lane mask 的两处修复，**有可能是 `f133e35e` 的解**（`a lane mask exchanged where not every lane runs`）。试挑过：4 处冲突，且依赖上游新建的 `read_phis` / `lane_edge` / `uniform_join` 结构，不是能顺手移植的，需要单独一轮。

---

