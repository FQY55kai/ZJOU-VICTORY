#!/usr/bin/env bash
# The switch block every number in docs/perf.md was measured with.
#
# `scripts/play.ps1` is the launcher for playing; this is the one for measuring,
# and it is kept separate so that a tuning change to one does not silently move
# the baseline of the other. Paths are yours to pass:
#
#   VICTORY_GAME_DIR=/path/to/dump VICTORY_FSR_DLL=/path/to/amd_fidelityfx_upscaler_dx12.dll \
#       bash scripts/run-perf.sh run
#
# `run` is the Series S fast path. For Series X use `--console series-x` below;
# its world scene is about twice as long a frame on the same machine.
set -u

GAME_DIR="${VICTORY_GAME_DIR:?set VICTORY_GAME_DIR to the title dump}"
GAME="$GAME_DIR/game_scarlett_master.exe"
[ -n "${VICTORY_FSR_DLL:-}" ] && export VICTORY_FSR_DLL

export VICTORY_GAME_DIR="$GAME_DIR"
# Pools: 32 GB of system RAM.
export VICTORY_LIST_POOL_MB=2048 VICTORY_LIST_POOL_MOST=2048 VICTORY_TEXTURE_STAGE_MB=512
# Video memory: a 12 GB card (upstream's defaults assume 16 GB: 4096/1536).
export VICTORY_CARD_WINDOWS_MB=4096 VICTORY_CARD_BUFFERS_MB=1536
# Upstream's play defaults.
export VICTORY_BAKE_TEXTURES_MS=10000 VICTORY_EARLY_TEXTURES_MS=40
export VICTORY_WAVE_OWN_LDS_GROUPS=64,128 VICTORY_WIDE_MERGE=1
export VICTORY_RELAY_OWN_QUEUE=1 VICTORY_UNDER_WRITES_WORDS=1
export VICTORY_UNDER_WRITES_RUNS=100000 VICTORY_UNDER_WRITES_MASKS=1
export VICTORY_RELAY=2 VICTORY_WIDE_REFRESH=1 VICTORY_ASK_ONCE=1
export VICTORY_TEXTURE_NO_WAIT=1 VICTORY_CARD_REHASH_RUNS=1 VICTORY_JOIN_COPIES=1
export VICTORY_D3D12_BACKGROUND=0 VICTORY_LATE_WAIT_LISTS=1024
export VICTORY_WINDOW_TAILS=1 VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_ASK_QUIET=8
export VICTORY_COPY_CHANGED=1 VICTORY_QUIET_BACK=1 VICTORY_WIDE_THREADS=12
export VICTORY_UNDER_WRITES_MASKS_LEAST=1
export VICTORY_CLAMP_AFTER_WRITER=1 VICTORY_CLAMP_BATCH=1 VICTORY_THIN=1
export VICTORY_WATCH_4K=1 VICTORY_INDIRECT_RAW=3 VICTORY_TEXTURE_HASH_EVERY=32
export VICTORY_LEVELS_ON_DEMAND=1 VICTORY_WINDOW_KB=1024 VICTORY_CARD_WINDOW_UPLOAD=1
export VICTORY_DROP_DXIL=1 VICTORY_DESC_DATA=1
export VICTORY_VRAM_IDLE_MS=15000,2500,1000 VICTORY_PIPELINE_WAIT_MS=2500
export VICTORY_DISCORD=0 VICTORY_OCEAN_READBACK=1
# The scripted run: no controller, no capture, presses A 150 times to get into
# the world, and dumps one CSV row per summary interval.
export VICTORY_AUTO_A=150
export VICTORY_FRAMETIME_DUMP="${VICTORY_FRAMETIME_DUMP:-frametime.csv}"

EXE="${VICTORY_EXE:-target/release/victory.exe}"

case "${1:-run}" in
  warm)  exec "$EXE" warm ;;
  run)   exec "$EXE" run "$GAME" --execute --console series-s ;;
  *)     exec "$EXE" "$@" ;;
esac
