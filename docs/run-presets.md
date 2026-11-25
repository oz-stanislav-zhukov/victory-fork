# Run presets (exact env per run)

Common: `VICTORY_BVH_BUILDER=0 VICTORY_AMBIENT_FILE=F:\Work\victory-runlogs\ambient.txt VICTORY_INDIRECT_WINDOW=1 VICTORY_SHADER_OPT=0 VICTORY_BVH_QUARANTINE=1`
Launch: `F:\Work\victory-fork\target-main\release\victory.exe run 'F:\gta6\GTA6.Xbox.v1.0.2.1-CYBERLEEK\game_scarlett_master.exe' --execute`

| Run | Branch | Extra env | Cell fps | GPU / copy-back ms | Picture |
|---|---|---|---|---|---|
| Z7 | fork-rays | `COPY_PLAIN=1 RELAY=1 LATE_WAIT=3 WIDE_BACK=1 CARD_BUFFERS_MIN_MB=1 CARD_BUFFERS_MB=1024` | 3.3-3.7 | ~172 / ~174 | bright, best so far |
| Z9 | fork-fast | Z7 + `ASYNC_SUBMIT=1 QUEUE2=1 TEXTURE_NO_WAIT=1 MERGE_GAP=64 MERGE_RUNS=4096 INDIRECT_GATHER=1 JOIN_PARTS=1 BACK_OWN_VIEW=1` | 4.7-5.1 | 109-118 / 41-51 | DARK cell, white blow-out |
| Z10 | fork-fast (= branch save-z10) | VItoriaBr 5060 Ti preset: `RELAY=2 QUEUE2=1 ASYNC_SUBMIT=1 COPY_PLAIN=1 TEXTURE_NO_WAIT=1 CARD_BUFFERS_MB=2048 TEXTURE_BUDGET_MB=10240 PAGE_WRITES=slots WIDE_REFRESH=1 WIDE_MERGE=1 JOIN_COPIES=1 DISPLAY_MODE=1920x1080` | 3.3-3.5 | 180-195 / 41-47 | bright; waitress face speckle in cutscene, eyes glow |

Conclusions: darkness in Z9 = one of `BACK_OWN_VIEW` / `JOIN_PARTS` / `INDIRECT_GATHER`. Fast copy-back comes from `QUEUE2`+`ASYNC_SUBMIT` (Z10 has it without BACK_OWN_VIEW). Z10's slow GPU likely `RELAY=2` instead of `RELAY=1 LATE_WAIT=3 WIDE_BACK=1` (unverified).
Next (Z11): build ccbba7ff (fork-fast + VItoriaBr 5a84eb41). Z10 preset but `RELAY=1 LATE_WAIT=3 WIDE_BACK=1 CARD_BUFFERS_MIN_MB=1` instead of `RELAY=2`. Then add `INDIRECT_GATHER`, `JOIN_PARTS`, `BACK_OWN_VIEW` back one per run.
