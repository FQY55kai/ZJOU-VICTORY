# 源码（victory-scarlett / scarlett-fox 分支）

本目录是通用包内 `victory.exe` 的完整源码，便于自行编译、审计和二次修改。

## 来源

- 上游：`https://github.com/Op-lite/victory-scarlett`，分支 `scarlett-fox`
- 快照：commit `bc264a4`（2026-11-27），含冻结修复（SOFT_VIEWS 页映射）、启动器着色器预翻译、画面修复（关 RT 无紫色植被）
- 谱系：`Marvisak/VIctory` → `fxceless/victory-fork` → `Op-lite/victory-scarlett`

## 编译

需要 Rust 1.88+（`rust-toolchain.toml` 已指定）：

```
cargo build --release --locked --bin victory
```

产物：`target/release/victory.exe`

## 与上游的差异

本仓库源码**未做代码修改**，与上游 `bc264a4` 一致。发布包里的差异全部在启动脚本的参数层：

- 多显存档位（8/12/16GB+）与预算（12GB 用 8192MB、ROOM 90）
- 缓存/存档物理隔离（`VICTORY_CACHE_DIR` / `VICTORY_DATA_DIR`）
- 补上游启动器默认的画面修复开关：`VICTORY_PROBE_RAYS=1`、`VICTORY_PLANE_NOW=1`
- 5 步启动流程脚本、10 分钟自动跑图看门狗、存档备份/还原

## 校验

发布包 `victory.exe` 的 SHA256：`3793dd8ab7cca14602d4eae6045e7137dc6367dc126d0e9e63eec134124d2c65`

用本目录源码自行编译出的 exe 大小可能略有差异（编译器版本/路径信息），功能等价。

## 许可

遵循上游 `LICENSE`（GPL-3.0-or-later）。本目录仅包含兼容层源码，**不包含任何游戏文件**。
