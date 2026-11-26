# 真 RT GI：需求与步骤

### 八、招募：真 RT GI（光线追踪反弹光照）—— 需求与步骤

**问题**：人脸与室内是黑的。截图证据：贴图、直射阳光全部正常（Lucia 头发/衣服材质完好），但阳光照不到的地方（脸的背光面、室内）几乎纯黑——缺少光追反弹光照（RTGI）。这是全社区（VIctory / victory-fork / victory-amd 各 README 一致承认）都缺失的最后一块大拼图：**`ray-traced bounce light does not run`**。

**为什么它值得做**：这是从"能跑"到"好看"的分水岭。做完之后阴影不再是死黑，画面观感接近主机。

> **2026-11-25 夜更新：这条路已经有人走通了。** PortWatch 榜首 `DexNom/VIctory`（78%，RX 7900 XTX）已跑通光追光照，做法与我们下面的判断一致——**不映射到 DXR**，而是翻译标题自己的 52 个 BVH 构建程序与 64 个遍历程序，只把 `image_bvh*_intersect_ray` 一条指令用 DXIL 模拟（详见其 `docs/fork-ray-tracing.md`）。
>
> **2026-11-26 凌晨更正**：查证后发现这套着色器代码**早就在我们树里**（`6a2d9f99` 已是祖先，`intersect.rs` / `own_bits.rs` / `wave64_tests.rs` / `docs/fork-ray-tracing.md` 全在且已接线）。缺的不是代码，是 `pipeline.rs` 里 `VICTORY_BVH_BUILDER` 那道硬门，而它注释里写的解锁条件（"遍历结构的程序能翻译"）现在已经满足。证据与下一步见**第十章**。

#### 我们已经有的（地基是实的）

| 组件 | 状态 |
|---|---|
| `XboxBvhBuilder.dll` 原生运行 | ✅ BLAS/TLAS 构建在跑（标题的 BVH 数据包 `core_bvh4*.rpf` 经 DirectStorage 正常加载） |
| BVH 内存布局 | ✅ 标题把 BVH 放在自己内存里（xmem 的地址范围约束正是为它：`the title's BVH builder refuses one above`） |
| 描述符"只取地址"路径 | ✅ 翻译器已有：`a buffer view the code only takes the address of, to walk the memory behind it (a ray-tracing structure)`——识别出了 BVH 遍历着色器 |
| 双精度/波求和 | ✅ phase2 已支持（水/大气的透射率、体积计算） |
| 部分光照累积着色器 | ✅ 已能翻译（deferred 面、水洼、浅水） |

#### 缺什么（按证据推断，从易到难）

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

#### 验收标准

- 序章囚室截图：Lucia 脸的背光面、背景人物、室内墙壁**不再是纯黑**（有可辨的环境/反弹光）；
- 帧率不低于当前基线（3fps 起步，预期 RT 开销可控在 2 倍内）；
- 连续运行 10 分钟无设备丢失。

**如果你要做这个，从 A 开始，把 A 的结论（软件遍历 or 硬件 TraceRay）发 issue，再动手。** 欢迎认领。

---

