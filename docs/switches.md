# 开关速查

### 六、关键开关速查

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
| `VICTORY_AMBIENT` | 间接漫反射光常量（0=全黑，`off`=保留原输出） | 默认 `0.6`；觉得偏亮可下调 |
| `VICTORY_BVH_BUILDER` | **开光追构建器**（标题 52 个 BVH 构建程序 + 约 1,000 个 resolve 程序） | 默认 `0`；开闸首跑要多编译 1.4–2.5 小时，见第十章 |
| `VICTORY_BVH_QUARANTINE` | 构建器产出只留宿主副本、不回写 title 内存 | 首次开闸配 `VICTORY_BVH_BUILDER=1` 用，防写坏 title 对象 |
| `VICTORY_BVH_RESOLVE=0` | 构建器开着时**先不造**那约 1,000 个 resolve 程序 | 首次验证构建器专用，把 1.4–2.5 小时压到 3 分钟 |
| `VICTORY_WAVE_WIDE` | 打开 `widen()`（发散分支整波一起走） | **默认 `0`；`=1` 实测会挂卡，别开**（见第十章） |
| `VICTORY_RT_RESOLVE` | 单独造 resolve 管线（不开构建器时它们没有命中可着色，纯浪费） | 默认 `0` |
| `VICTORY_WAVE_WHY` | 打印 wave64 重试每一步（`own_bits` / `WAVE64-WIDE`） | 诊断光追翻译时开 |
| `victory warm` | 着色器预热 | 更新构建后跑一次 |

#### 操作方式（不接手柄时）

| 输入 | 手柄 | 说明 |
|---|---|---|
| **W A S D** | 左摇杆 | 移动 |
| **鼠标移动** | 右摇杆 | 转镜头（停下即停，不漂移） |
| **I J K L** | 右摇杆 | 键盘转镜头（备用） |
| **鼠标左键** | 右扳机 | 开火/攻击 |
| **鼠标右键** | 左扳机 | 瞄准 |
| **鼠标中键** | A | 交互/确认 |
| **Enter / 空格** | A | 交互/跳跃/确认 |
| **Backspace** | B | 返回 |
| **X / Y** | X / Y | X、Y 键 |
| **方向键** | 十字键 | 菜单导航 |
| **Q / E** | 左右肩键 | 切换 |
| **Z / C** | 左右扳机 | 油门/刹车 |
| **Esc / Tab** | Menu / View | 暂停/切换视图 |

---

