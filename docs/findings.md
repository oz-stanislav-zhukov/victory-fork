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
| Z10 | fork-fast (= save-z10) | VItoriaBr 5060 Ti preset (RELAY=2, 1080p, PAGE_WRITES=slots ...), no BACK_OWN_VIEW/JOIN_PARTS/INDIRECT_GATHER | 3.3-3.5 | 290-306 (GPU 180-195!, copy-back 41-47) | BRIGHT cell again -> Z9 darkness was one of BACK_OWN_VIEW/JOIN_PARTS/INDIRECT_GATHER. Fast copy-back comes from QUEUE2+ASYNC_SUBMIT. Slow GPU likely RELAY=2. Waitress face speckle in the opening cutscene, eyes glow. heap_buffers 28-65 ms/frame | exact env in docs/run-presets.md |
| Z9 | fork-fast (fork-934 + #1002 + fxceless main d510ee4a) | + ASYNC_SUBMIT QUEUE2 TEXTURE_NO_WAIT MERGE_GAP=64 MERGE_RUNS=4096 INDIRECT_GATHER JOIN_PARTS BACK_OWN_VIEW | **4.7-5.1** (window 4.1) | 195-212 (GPU 109-118, copy-back 41-51!) | FASTEST so far, but cell dark again (ambient logged on) and a cutscene shows a huge white bloom + rainbow streak. Suspect: exposure the title reads back arrives late/stale with the new back-copy switches (BACK_OWN_VIEW first) | bisect switches |
| Z8 | fork-skin (fork-rays + #934 + #927) | shaders changed (warm-up) | 3-4 | 244-385 | guard's forearms CLEAN at B35 (fully rainbow in Z7): #934/#927 fix the skin noise. Waitress face rainbow only in the opening cutscene (cold cache?). Lucia's eyes STILL glow (#934 not the eye fix). New: grey blotches on cell walls/ceiling and one corridor wall, stay with AO back at default -> from #927 or #934 (fork-934 built to split) | best for skin |
| Z7 | fork-rays (+ambient counters) | builder OFF, guard ON | 3.3-3.7 cell | 267-306 | user: best so far. Waitress face CLEAN (speckled in Z6, clean in Z3: intermittent). Lucia's/waitress eyes still glow. Faces and some objects flicker black now and then. Corners lack occlusion -> live AO raised (ambient.txt 0.5 1 0.98 0.94 2.2 1.2 0.1 0.45, default kept in ambient-default.txt) | guard back on = textures 4-32 ms again |
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
| Black corridor walls = strong ambient AO | Z4, AO back to default | REFUTED (walls stayed black) |
| Black walls are time-dependent (degrade later in a run) | Z6, user | REFUTED: in Z6 the cell-block corridor stays fine all run (was black in Z4); the big common hall (A102/A103, first mission) is black with white patches. Area/material dependent. Corridor fix in Z6 = guard off, #148 or #994 (not yet split) |
| Darkness with builder on = ambient switched off | Z5 log `ambient ... shaded=true` | REFUTED: ambient on; ray passes occlude it |
| Builder crash at world entry (0xffff... read) | Z5 with #994 | FIXED by #994 (thread-count dispatches) |
| Which change fixed the black walls | Z4 (guard on, no #148/#994): corridor black. Z6 (guard OFF, +#148/#994): corridor fine, hall black. Z7 (guard on, +#148/#994): corridor AND hall fine | SETTLED: #148 or #994 fixed the corridor; guard OFF caused the black hall in Z6. Keep guard on. Hall with 10-11k draws: 3 fps / 316-342 ms in Z7 vs 2 fps / 443 ms in Z6 |
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
| fxceless #1002 | merged (fork-934, fork-fast) | tiled sun-shadow programs exact on 32-lane: orange blocks on hair |
| fxceless main 465b94f5, 39731795, ca8829ff, d510ee4a | merged (fork-fast) | INDIRECT_GATHER, JOIN_PARTS, BACK_OWN_VIEW: 9.6-10.3 fps in the first rooms on their RTX 4070 |
| Candidates: fxceless #995 #1014 #1013 #1006 #934 #927, Zettrox #99, Belobug 72e748ea | pending | see Qwen research reports |

## Open problems
1. Waitress (and cellmate, guard arms) skin rainbow speckle: not the texture guard, intermittent (Z3 clean, Z6 speckled, Z7 clean). Qwen report 01: v_interp_p2_f16 always writes the low half (lower.rs low_half), fixed by fxceless #934; #927 notes an intermediate form gave 'rainbow squares on the waitress' skin'. Both merged on branch fork-skin, to run next.
1b. Faces and some objects flicker black for a frame (Z7). Ambient counters (seen / replaced / indirect) in the log will tell if the resolve sometimes runs unreplaced.
2. Lucia's eyes glow white/blue in close-ups. Qwen report 02: same #934 (v_interp_p2_f16 drops clamp/omod: values >1 blow out white); #948/#1019/#927 unlikely. Test on fork-skin.
2b. Z7: guard's forearms fully rainbow at door B35 (face clean): skin noise is per shader variant, not per run.
3. Black walls with white 'peeled' patches in the big common hall (Z6); the corridor was black in Z4 but fine in Z6. Area/material dependent; which Z6 change fixed the corridor is not yet split. User (Z6): a table in the hall goes black and comes back: flicker, so lighting/ambient per frame or re-uploads, not a broken texture.
4. Orange 8x8 blocks on Lucia's hair (common room, Z6); Z7: blocky squares over the hall ceiling, lamps and windows in one shot (light/volumetric pass tiles?).
5. Rays: builder runs but darkens the image and doubles GPU time. Qwen 04: tree is partly garbage (our translator drops s_waitcnt: Belobug 72e748ea adds UAV fences around atomics; empty +-inf boxes occlude everything, fxceless #16); quarantine leaks (#1013). Even fixed, ~88 ms more GPU a frame: parked while fps is the priority.
5b. Hair blocks: Qwen 03 -> fxceless #1002 (CS_ShadowRenderTiled* on 32-lane hosts, matches fxceless #62 word for word). Merged on fork-934/fork-fast.
6. fps 3-4: copy-up ~90, GPU ~150, copy-back ~170 ms.

## Clean-up log
| Date | Removed | Why |
|---|---|---|
| 2026-11-25 | git stash (2 entries) | both only a build-regenerated Cargo.lock |
| 2026-11-25 | target-main/release/victory-runY2/Z/Z2/Z3.exe | branches pushed, can be rebuilt; kept Z4 (last base without #993/#994) and Z5 |
| 2026-11-25 | F:\Work\victory-fork\target-jman (807 MB) | JMAN tree not used (only a warning about da125a4) |
| 2026-11-25 | git stash on fork-skin (auto, branch switch) | only the build-regenerated Cargo.lock |
| 2026-11-25 | 34 run logs (run..runZ3) -> archive\runs-A-to-Z3.zip (171 MB -> 36 MB folder) | conclusions are in this file and agent-log; raw kept zipped |

## Posted to the community
| Date | Where | What |
|---|---|---|
| 2026-11-25 | fxceless #148, #891, #917, #950 (x2) | Windows 10 results, #854 builds all pipelines, ambient + shade on NVIDIA, #854 for Arc hangs |
| 2026-11-25 | [fxceless #652](https://github.com/fxceless/victory-fork/issues/652#issuecomment-7169362050) | `TEXTURE_GUARD=0` costs 40-60 ms here, does not clear skin noise, blackens the hall |
| 2026-11-25 | [fxceless #994](https://github.com/fxceless/victory-fork/pull/994#issuecomment-7169362576) | builder runs 11 min on Win10/5060 Ti without CBV cap, but darker and GPU 172->272 ms |

## TODO
- In-game stats panel like "Kitty Runtime" (user screenshot, 2026-11-25): title + big fps, then a row FRAME / GPU WAIT / CPU PREP / COPY-BACK in ms, then draws/frame, dropped/frame, RAM. Base: `crates/victory-gpu/src/fpsbox.rs` (fxceless ca8829ff, fps drawn into the top-left corner, `VICTORY_FPS_COUNTER=0` off). The numbers already exist in the frame line (host.rs "fps, ms/frame (preparing ...; on the GPU; copying back ...)"). Make it a small dark panel with labels; switch to toggle.
- Split #927 vs #934 for the grey wall blotches (fork-934 has #934 + #1002 without #927).
- Ambient counters threshold 2000 too high (never printed in 13 min): lower to ~200.

## Other forks checked
| Fork | Base | Verdict for us (NVIDIA 32-lane, Win10) |
|---|---|---|
| uiliannn/VItoriaBr (180 stars) | fxceless 0b0074e + ports (Belobug ambient, Zettrox #148, DexNom 921d8d3/bc3cd4b, CBV clamp 64 KiB #28, heap_buffers CPU cut 5a84eb41) | MOST USEFUL: README preset tested on an RTX 5060 Ti 16 GB (our card), up to 6 fps, 'no black skin, no TDR'. All 13 switches exist in fork-fast. Key ones we never used: `DISPLAY_MODE=1920x1080` (we render 2560x1440!), `PAGE_WRITES=slots` (re-queues textures uploaded before the title finished writing: 7,000+ a session; may be our black flicker), `CARD_BUFFERS_MB=2048` + `TEXTURE_BUDGET_MB=10240`, `RELAY=2`, `WIDE_REFRESH/WIDE_MERGE/JOIN_COPIES=1`. Also `SHADER_LOOP_LIMIT>=4096` for hair (issue #1). Worth porting later: 5a84eb41, CBV clamp (needed for the builder, fxceless #911). |
| DexNom/VIctory (PortWatch #1, 78%, 2026-11-25) | Marvisak/VIctory + ZSlayerHQ victory-world-runtime, Radeon-focused; issues disabled | Not ahead on NVIDIA: their own NVIDIA reports (PR #14 RTX 3060 Ti: ~2 fps, 517 ms; PR #15 RTX 4070: 1-1.4 fps, 700-1000 ms), RT cannot start on 32-lane (8 pipelines refused, builder missing=5 of 53). We: 4.7-5.1 fps (Z9) and the builder runs (#854 + #994). Different tree (Marvisak), so ports are big. Idea worth knowing: cd94d353 view marks (shaders set a bit per 64 KiB page they write; copy-back compares only marked pages: was 2.8-4.5 GB/frame). Their PR #12 (Agility SDK at runtime for Win10) we already have via #22. |

## Where we stand (2026-11-25, end of session)
- Best picture: fork-skin / fork-934 line (Z7-Z8): bright cell, walls fine, skin noise gone on the guard's arms (#934/#927). Best speed: fork-fast + speed switches (Z9): 4.7-5.1 fps, but the cell is dark again and cutscenes blow out white.
- NEXT (Z11), built and waiting: fork-fast ccbba7ff (+ VItoriaBr 5a84eb41 heap_buffers CPU cut). Env: Z10 preset but RELAY=1 LATE_WAIT=3 WIDE_BACK=1 CARD_BUFFERS_MIN_MB=1 instead of RELAY=2. Then add back INDIRECT_GATHER, JOIN_PARTS, BACK_OWN_VIEW one at a time to find the darkening one.
- DONE Z10: fork-fast with VItoriaBr's 5060 Ti preset, WITHOUT our Z9 extras (BACK_OWN_VIEW, JOIN_PARTS, INDIRECT_GATHER): RELAY=2 QUEUE2=1 ASYNC_SUBMIT=1 COPY_PLAIN=1 TEXTURE_NO_WAIT=1 CARD_BUFFERS_MB=2048 TEXTURE_BUDGET_MB=10240 PAGE_WRITES=slots WIDE_REFRESH=1 WIDE_MERGE=1 JOIN_COPIES=1 DISPLAY_MODE=1920x1080, plus AMBIENT_FILE, SHADER_OPT=0, BVH_BUILDER=0, INDIRECT_WINDOW=1. Bright picture -> darkness was one of the three extras; also the 1080p speed gain.
- NEXT 1: bisect Z9's switches for the darkness, one per run, same fork-fast build: drop BACK_OWN_VIEW first, then JOIN_PARTS, then ASYNC_SUBMIT+QUEUE2, then INDIRECT_GATHER. If none: fork-fast without switches (the code itself: fxceless main or #1002).
- NEXT 2: split #934 vs #927 (grey wall blotches): fork-934 run with base switches.
- NEXT 3: stats panel (TODO), ambient counter threshold, Qwen reports 05b-09.
- Qwen research loop keeps running (F:\Workictory-runlogs\qwenesearch, STOP file stops it).
