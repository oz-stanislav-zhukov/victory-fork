# Findings: what is settled, so nobody re-checks it

Machine: RTX 5060 Ti 16 GB (32-lane waves only), i7-14700KF, Windows 10 + Agility SDK. Dump: GTA6 Xbox 1.0.2.1.
Chronological detail is in `agent-log.md`. Raw logs: `F:\Work\victory-runlogs\` (Z4 and later), older ones zipped in `archive\runs-A-to-Z3.zip`.

## Base run switches (all runs since runZ unless noted)
```
VICTORY_AMBIENT_FILE=F:\Work\victory-runlogs\ambient.txt   (0.6 1 0.98 0.94 1 0.6 0.1 0.45)
VICTORY_INDIRECT_WINDOW=1 VICTORY_COPY_PLAIN=1 VICTORY_SHADER_OPT=0 VICTORY_BVH_QUARANTINE=1
VICTORY_RELAY=1 VICTORY_LATE_WAIT=3 VICTORY_WIDE_BACK=1 VICTORY_CARD_BUFFERS_MIN_MB=1 VICTORY_CARD_BUFFERS_MB=1024
VICTORY_BVH_BUILDER=0 (unless noted)
```

## Runs (newest first)
| Run | Branch | Change vs previous | fps (world) | ms/frame | Picture | Verdict |
|---|---|---|---|---|---|---|
| Z6 | fork-rays (+#148) | builder OFF, `TEXTURE_GUARD=0` | 3-4 (median 3.4); hall 2 | 243-385; hall 443 (9.6k draws) | bright again; corridor walls normal (were black in Z4); waitress face STILL rainbow; Lucia eyes glow; orange 8x8 blocks on Lucia's hair in common room | fork-rays = new base. Guard is not the skin noise |
| Z5 | fork-rays | #993+#994, builder ON | 2.4-3.0 | 335-460 | much darker, GPU ~270 ms, copy-back ~280 ms | builder runs without crash, but rays not worth it yet |
| Z4 | fork-pool | +#959 | 3.7-4.1 | 244-273 | corridor walls black + white patches; skin speckle | no crash in 10+ min |
| Z3 | fork-shade | ambient shade (depth AO) | 3-4 | 258-300 | corners/joints darker, less flat | kept |
| Z2 | fork-ambient | Belobug ambient | 3-4 | | cell lit, colour blocks gone | kept (big win) |
| Z | fork-next | fxceless main a253706+0b0074e | | | lamps lit | kept |
| Y2 | fork-speed | #834 | 3.6-3.8 | 265-280 | bottom band gone | kept |
| X | fork-speed | SHADER_OPT on | 3.6-3.8 | | cell much darker | `VICTORY_SHADER_OPT=0` stays |

## Hypotheses tested (do not re-test)
| Hypothesis | Test | Result |
|---|---|---|
| Skin rainbow speckle = stale textures the texture guard misses | Z6 with `VICTORY_TEXTURE_GUARD=0` (hash every check) | REFUTED: waitress face still speckled. Look at the skin shader instead |
| `VICTORY_TEXTURE_GUARD=0` is free (fxceless#652: "not slower") | Z6 vs Z4 frame lines | FALSE here: textures 41-51 ms / 170-220 MB a frame vs 6-26 ms / 25-85 MB; preparing 135-144 vs 72-95 ms; cell 3.2-3.6 vs 4.0-5.3 fps; big hall (9.6k draws) 2 fps. Keep the guard ON |
| Black corridor walls = strong ambient AO | Z4, AO back to default | REFUTED (walls stayed black). Walls fine in Z6 (guard off and/or #148/#994): not yet split which |
| Darkness with builder on = ambient switched off | Z5 log `ambient ... shaded=true` | REFUTED: ambient on; ray passes occlude it |
| Builder crash at world entry (0xffff... read) | Z5 with #994 | FIXED by #994 (thread-count dispatches) |
| #2473 (Marvisak) affects us | code read | NO: needs `VICTORY_REFRESH_ONCE=1`, off here |
| #2414 (Marvisak) affects us | code read | NO: our fxceless watch.rs has no alias-following `changed` |
| JMAN da125a4 (sampler mip-none) | runs | HARMFUL: whole image blurry; reverted |
| Dropped ~320 draws/frame | diagnostics | = BVH builder stages while builder is off; expected |

## PRs and commits evaluated
| Item | Status | Why |
|---|---|---|
| fxceless #854 | merged | 0 refused pipelines, 0 untranslated programs |
| fxceless #834 | merged | removes bottom band |
| fxceless #959 | merged | pool crash fixes (not proven needed here) |
| fxceless #993 + #994 | merged (fork-rays) | builder no longer crashes; kept off by default |
| Zettrox #148 | merged (port by Qwen, verified identical) | descriptor allocator refused tables that fit |
| Belobug ambient + shade | ported | lights the cell, removes colour blocks |
| Belobug PR 128/68/138 | skipped | same as #854 |
| Zettrox PR 116 | skipped | same ambient we have |
| fxceless #999 | skipped | water/boat texture readback, not our issue |
| JMAN #533 | skipped | old upstream issue, closed there |
| Belobug ASYNC_SUBMIT (bdecda30, cf57ca33, 9b89ec2d) | pending | conflicts in host.rs/list.rs/queue.rs |
| Candidates: fxceless #995 #1014 #1013 #1006 #934 #927, Zettrox #99, Belobug 72e748ea | pending | see Qwen research reports |

## Open problems
1. Waitress (and cellmate, guard arms) skin rainbow speckle: not the texture guard. Suspect skin/SSS shader.
2. Lucia's eyes glow white/blue in close-ups.
3. Orange 8x8 blocks on Lucia's hair (common room, Z6).
4. Rays: builder runs but darkens the image and doubles GPU time.
5. fps 3-4: copy-up ~90, GPU ~150, copy-back ~170 ms.

## Clean-up log
| Date | Removed | Why |
|---|---|---|
| 2026-11-25 | git stash (2 entries) | both only a build-regenerated Cargo.lock |
| 2026-11-25 | target-main/release/victory-runY2/Z/Z2/Z3.exe | branches pushed, can be rebuilt; kept Z4 (last base without #993/#994) and Z5 |
| 2026-11-25 | F:\Work\victory-fork\target-jman (807 MB) | JMAN tree not used (only a warning about da125a4) |
| 2026-11-25 | 34 run logs (run..runZ3) -> archive\runs-A-to-Z3.zip (171 MB -> 36 MB folder) | conclusions are in this file and agent-log; raw kept zipped |
