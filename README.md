# ZJOU-VICTORY

## This branch: victory-amd + a DirectStorage read fix

A fork of [myokex/victory-amd](https://github.com/myokex/victory-amd) at `070df42`
with one change on top (on this fork's `victory-amd` branch, and on `fix-dstorage-guard-race`):

- **Kernel reads into title memory are read beside and copied in.** With the guard
  over all the views of a page (on by default in `070df42`), a raw DirectStorage
  request could fail with `ERROR_INVALID_USER_BUFFER` (`0x800706f8`) and the title
  stopped itself seconds after the world's first frame: 2 runs out of 2. Those reads
  now land in a buffer and are copied in from user mode, where the guard takes the
  write; `fread` does the same. With the fix and the guard on, a run went 5.5 minutes
  past the world's first frame without a failed request.

Tested on one machine only: Windows 11, Radeon RX 7800 XT 16 GB (driver
32.0.32015.7029), Ryzen 7 7700X, 32 GB RAM, Series X profile. Other GPUs,
NVIDIA's especially, may behave differently.

**No game files, code or assets are here, and none are shared.** Shaders are
translated on your machine from your own copy of the game.

### Reproducing the check run

1. Install Git, Visual Studio Build Tools (MSVC C++ workload and a Windows 10/11 SDK),
   [rustup](https://rustup.rs), and a current GPU driver. The pinned Rust toolchain is
   installed by itself on the first build.
2. Get the code and set up the config:

   ```powershell
   git clone https://github.com/JoshuaO22/victory-amd
   cd victory-amd
   copy victory.example.toml victory.toml   # set game_dir to your own dump, outside the repo
   cargo test --workspace                   # optional; needs no game data
   cargo build --profile diag -p victory --features diagnostics
   ```

3. **Recommended, about 4 minutes: translate the shaders ahead of time.** Without it,
   each shader is translated the first time the title uses it, and what it draws is
   left out until then: the opening cutscene and the world's first minutes show missing
   objects and effects, and run slower. With it, every shader is ready from the first
   frame. The translations are made on your machine from your own copy of the game, and
   are kept in the cache for later runs. Use the same build as the runs (the cache key
   includes it) and no shader-related `VICTORY_*` switches:

   ```powershell
   target\diag\victory.exe warm
   ```

   Run it again after updating this repository: a change to the translator makes the
   cached translations stale, and they are made again.

4. Run the title with a fresh data folder and a log:

   ```powershell
   $env:VICTORY_LOG = 'info'
   $env:VICTORY_DATA_DIR = 'D:\victory-data\check-1'   # any new folder outside the repo
   Start-Process target\diag\victory.exe -NoNewWindow -Wait -RedirectStandardError check-1.log `
     -ArgumentList 'run', '"<game dir>\<the title''s exe>"', '--execute', '--console', 'series-x'
   ```

   Go through the first-time setup and legal screens with a controller; they do not
   advance by themselves. The test window is the first minutes after the story's world
   starts loading. Let it run about 7 minutes, then close it.
5. Passed if `VICTORY_DATA_DIR\<title id>\crashes.log` does not exist and the log has no
   `dstorage: request failed` line. The log names the world's start in a line with
   `the first frame of 500 draws or more`. Without the fix, the same run stops within
   about 25 seconds of that line.

To repeat it without a controller: copy `saves` and `storage` from the data folder of a run
stopped before the first autosave into a new data folder. Setup and legal screens are then
done, and the story starts from the beginning again.

## victory-amd

This copy is a fork of
[fxceless/victory-fork](https://github.com/fxceless/victory-fork), itself a fork
of [VIctory](https://github.com/Marvisak/VIctory). It adds fixes found while
running on an AMD card, tested on one machine only: Windows 11, Radeon RX 7800 XT
16 GB (driver 32.0.32029.2010), Ryzen 5 7500F, 32 GB RAM.

State on that machine (2026-11-25, Series X profile): the prologue's cutscene and
the first room are drawn, with the player moving, at 2 to 3 frames per second.
Runs of 8 minutes mostly end without a device loss; one ended with
`DXGI_ERROR_DEVICE_HUNG` after about 5 and a half minutes, cause unknown. Lamps
were dark in one run. The first room's walls were missing in several runs; with the
guard over all the views of a page (below) they were there in two runs out of two,
too few to call it fixed. Some shots of the opening cutscene come out white.
Ray-traced light does not run.

What it changes over `victory-fork` (each is explained in a comment where it is):

- Texture views are told apart by the memory their mips really take, so textures
  streamed in part are not taken for one another and are uploaded from their first
  mip that is there.
- Views of placed textures the host lost sight of are put back
  (`VICTORY_TEXTURE_SWEEP`).
- Dispatches of more than 65,535 groups are made in two parts (the final gather at
  2560x1440 asks for 115,200).
- DirectStorage requests read their page array when they run, and wait for it.
- The list of pipelines to make early keeps what earlier runs listed, and compute
  pipelines get their own waiting time while a world loads.
- A write of the title's through one view of a physical page is seen by the copies
  taken through its other views (`VICTORY_VIEW_GUARD=0` turns it off).
- Textures picked by index are looked at again after their upload
  (`VICTORY_HEAP_RECHECK=0` turns it off).
- View indices past the host heap are clamped on AMD too.
- Programs that stop the AMD driver's compiler, or take it minutes and tens of GB,
  are left out (ray-tracing resolves).
- Diagnostics: `VICTORY_NULL_MAGENTA`, `victory warm`, more captures.

The guard over all the views of a page is code of
[victory-fork #115](https://github.com/fxceless/victory-fork/pull/115) by Spexivus
with [its correction](https://github.com/Spexivus/victory-fork/pull/1) by joroyt and
a part of [VIctory #1122](https://github.com/Marvisak/VIctory/pull/1122) by tightfall,
adapted here. The second look at textures is fxceless's idea (victory-fork `a803c87`),
written anew. Other parts were prompted by pull requests and issues of VIctory and
victory-fork, and their code was written anew. Developed with [Claude Code](https://claude.com/claude-code).

A work-in-progress fork of [VIctory](https://github.com/Marvisak/VIctory), a
compatibility layer that runs an Xbox Series title natively on Windows from
files the user supplies. This fork follows upstream
`main` and adds fixes aimed at one goal: getting the game's world to render
correctly and playably on one particular PC.

**This repository contains no game files or assets.** You need your own copy
of the game. Not affiliated with or endorsed by its developer or publisher.

## Status (2026-11-25)

Tested on Windows 11 with an **NVIDIA GeForce RTX 3050** (8 GB):

| | |
|---|---|
| Boot, first-boot screens, intro, opening credits | works |
| Prologue | playable from the cell to the boat chase: cell, cafeteria, robbery, police chase (wanted level, minimap), boat, binoculars |
| Stability | 1 h 30 min+ in one session without a crash |
| Frame rate | 1-3 fps in the world (the first run is slower while shaders translate; they are cached after that) |
| Known problems | anything in shadow is black and bright scenes overexpose (the game's ray-traced lighting does not run yet); some blocky artifacts; boats sit low in the water |

## Requirements

- Windows 10/11 x64, a CPU with AVX2
- A Direct3D 12 GPU with shader model 6.6 and mesh shaders (NVIDIA RTX 20 series or newer)
- Rust (the version in `rust-toolchain.toml`)
- Several GB of free Windows commit memory (raise the paging file if `crashes.log` reports error 1455)

## Build

```powershell
cargo build --release -p victory
```

## Play

```powershell
powershell -ExecutionPolicy Bypass -File scripts\play.ps1 -Game "<your game folder>\<the game's exe>"
```

`scripts\play.ps1` keeps the shader cache and saves in folders you choose
(`-Cache`, `-Data`) and presses no buttons for you. Controller: XInput pads
(a DualShock/DualSense through DS4Windows works). With no pad connected the
keyboard acts as one while the game window has focus.

## Faster settings (optional, off by default)

`VICTORY_RELAY=2`, or `VICTORY_LATE_WAIT=3 VICTORY_WIDE_BACK=1` cut frame
time on the test machines. Set them in PowerShell before running the script.

## Debugging switches

| Switch | Effect |
|---|---|
| `VICTORY_CAPTURE=<dir>` + `VICTORY_CAPTURE_FRAMES=<list>` | save those frames as PNG (diagnostics build) |
| `VICTORY_KEYS=30:Enter,35:X` | press keys at those seconds (diagnostics build) |
| `VICTORY_D3D12_DRED=1` | on a GPU failure, report the commands in flight |
| `VICTORY_PIPELINE_WAIT_MS=24` | shorter per-frame wait for pipelines still being built |

Diagnostics build: `cargo build --profile diag -p victory --features diagnostics`

## License

GPL-3.0-or-later (see `LICENSE`). Based on VIctory (Marvisak) and victory-fork (fxceless).
