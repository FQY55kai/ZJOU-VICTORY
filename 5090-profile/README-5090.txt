RTX 5090 32GB profile (2026/11/27)
fork: victory-scarlett-3070 "scarlett-fox"

1. Edit Start-5090-GTA6.cmd, set VICTORY_GAME_DIR to your game folder
2. First time: run Warm-shaders.cmd (~21 min, once)
3. Start via Start-5090-GTA6.cmd

5090 differences vs the 12GB verified combo:
- Frame clock 16.667ms = 60fps tier. For a higher cap lower the value
  (8.3 = 120fps). Delete the line to use the title's own 30fps.
- MIP_DROP=0: full-resolution textures, no mip dropping on 32GB
- TEXTURE_BUDGET_MB=20480 / ROOM_PERCENT=94: generous VRAM headroom
- CARD_BUFFERS_MB=2048 / CARD_WINDOWS_MB=1024: bigger staging

NOTE: fps ceiling is the CPU translation layer (25-40ms/frame on a
24-thread desktop CPU class machine), not the GPU. A 5090 removes the
VRAM/quality limits, not the translation bottleneck.
