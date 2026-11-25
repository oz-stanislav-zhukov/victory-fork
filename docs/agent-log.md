# Agent log (terse; read first, append after each run/fix)

HW: Win10 19045, RTX 5060 Ti 16GB drv 621.24 (wave32), i7-14700KF, 64GB. Title 1.0.2.1 SeriesX.
Branches: `win10-agility` = fxceless 4e17478 + PR#22 + PR#58 (pushed to origin=oz-stanislav-zhukov/victory-fork). `full-refresh` = win10-agility + Yorivel d6330a6 + own WIP (uncommitted).
Remotes: origin=own fork, fxceless (no-push), upstream=Marvisak (no-push).
Logs: scratchpad run*.log (temp). Legend: OK helped, NO no effect, COST helped at cost, ? unverified.

## Best config (runB2): ~3.2-3.4 fps @5k draws, 290-320 ms
SHADER_OPT=0 BVH_BUILDER=1 BVH_QUARANTINE=1 RELAY=1 LATE_WAIT=3 WIDE_BACK=1 CARD_BUFFERS_MIN_MB=1 CARD_BUFFERS_MB=1024 (all VICTORY_*)

## Facts
- main on Win10 crashes 0xc0000005 read 0x68 @+0xb5a5f8 (root sig flags c01). OK: PR#22 + `cargo xtask agility`.
- Intro jitter = texture guard 8-frame hash (#39). OK: PR#58.
- Speed switches: OK +25-30% (card busy 45%->82%). Frame now bound by GPU ~150-190ms + prep ~100-140ms + copy-back.
- Lighting: OK BVH_BUILDER=1+QUARANTINE=1 (static BVHs). Dynamic builder off: 3/53 progs fail (divergent readfirstlane).
- SHADER_OPT (1684989): NO-bad: black mosaic; =0 clean, same fps, GPU +15-20ms. Cache keyed by it: first run after change ~20 min black/drops. Issue #310 (related #120/#145/#195/#317/#122).
- Remaining with OPT=0: black eyes 1 closeup, blue noise "NINE MONTHS AGO", square patches on hair (#62 CS_ShadowRenderTiled*), broken room ceiling.
- Ceiling/walls missing = stale shadow copies (#62/#168). COST: FULL_REFRESH=1 fixes it, 0.4-0.6 fps (7.5-11 GB/frame copies).
- Slots 0x1c8/0x218 warnings: not a problem (#113).
- #356 tests: here min/max + wave64 pass; 4 intersect tests fail only "no SM 6.6" (test exe lacks Agility on Win10).
- Don't kill the game without asking the user.

## WIP fixes for stale copies (full-refresh branch)
1. NO: mark DirectStorage writes to physical pages (xmem.rs written/shown_at, PageWindow Drop). Ceiling still broken. Kept (correct, cheap). Test pages_written_are_found_at_every_address_that_shows_them.
2. Aliases: title maps same physical pages at several VAs; guard of one VA misses writes at another. watch.rs protect()/open() also guard/open alias blocks; inplace.rs aliases(). Log: >2048 alias blocks in 1st minute => real.
   - v1 NO-COST: full scan of views per call -> 0.1 fps (3.5-13 s/frame). Ceiling not checked.
   - v2 ?: by_section index (offset,addr)->len + longest per section. runF.
   - v2 NO: index made it ~1-3 fps (vs 3.2-3.4), ceiling still missing, and DOORS of the room vanished (regression). Reverted (inplace.rs, watch.rs). Aliases exist but guarding them is the wrong lever.
3. Next: bisect FULL_REFRESH by buffer (address/size range env) to find which shadow copies go stale; then trace who writes them.
4. Diag VICTORY_STALE_CHECK=1 (shadow.rs stale_check): after tracked refresh, memcmp copy vs title mem per 64K page, log "stale copy" (start,len,pages,carded) at power-of-2 counts, copy stale pages. runG.
- Display mode: title is told 3840x2160 by default. Fork already has VICTORY_DISPLAY_MODE=WxH (dxgi.rs display_size); upstream #1701 (VICTORY_MODE) is the same idea, no port needed. Next: run B + VICTORY_DISPLAY_MODE=1920x1080, compare GPU ms.
   - runG result: COST ceiling+walls back (like FULL_REFRESH), ~0.6-0.7 fps. 539 distinct buffers flagged. carded=true (e.g. 0x1af60000 174MB heap, 119 pages, 512+ times) + many carded=false 4MB buffers (0xce..-0xdc.., 1-3 pages). New artifact: blue speckle noise in bar scene => repairing carded copies clobbers shader-written card data => carded diffs likely noise. Next: STALE_CHECK=2 = non-carded only.
   - STALE_CHECK=2 (non-carded only) added.
- runH DISPLAY_MODE=1920x1080 (title told 1080p, window still 4K): GPU 186->154 ms avg (-17%), copy-back 198->166, fps ~same (3.1 vs 3.4, only 11 warm rows). NO big win: GPU not resolution-bound (compute/fixed-size passes). Ceiling looked intact in this run (stale copies are intermittent, #168) but room doors missing again; speckle mosaic on Yolanda's face. Reached next room + "talk to Yolanda" prompt (gameplay progresses).
- runI: STALE_CHECK=2 (non-carded only), B config, 4K.
   - runI result: ceiling back with non-carded only (=> culprits are non-carded copies). Bar speckle STILL present (=> not caused by carded repair; separate issue). ~1.5 fps (prep 640 ms memcmp). 619 stale buffers, 587 are exactly 4 MB; usually 1 stale 64K page, random offset 0..63.
   - Added to stale log: mapped (VirtualQuery MEM_MAPPED) and by_copy_memory (page hit by recent CopyMemoryToMemoryX CPU copy, list.rs copy_memory -> shadow::note_cpu_copy). Suspects: copy_memory (deferred ptr::copy, no watch::wrote) vs other. Next run tells.
   - runJ: stale buffers: 603x 4MB mapped=true, 160x 4MB mapped=false, by_copy_memory=0 for ALL => CopyMemoryToMemoryX NOT the cause.
- OTHER TEAM (Discord stream "Trying to merge XCL/Leonida/Diana", their origin/main 411a1dd, repo URL unknown/private): root cause of missing room = title maps same physical memory twice: one large view + ~130,000 small (64K) views, GPU addresses from both; copies watched only at the address taken; write via other view unseen -> stale forever. Fixed in their tree, cost +12-20 ms copying/frame (420-480 vs 370-450 ms). Matches our alias hypothesis; our v1/v2 (guard aliases) was the wrong implementation. Get their diff if possible.
5. Alias fix v3 (own, after other team's diagnosis): inplace.rs aliases() w/ by_section index restored; watch.rs open() -> note_alias_writes(block) pushes alias pages to ALIAS_WRITES (no guarding of aliases); writes() -> take_alias_writes(): j.asked+=1, journal pages written=now => copies taken earlier recopy those pages. Lock order: guards->ALIAS_WRITES/VIEWS; journal->ALIAS_WRITES. Unguarded alias VAs still undetected (log counts). runK pending.
   - runK v3 result: NO. Ceiling intact but black face in bar, slower (383-515 ms vs ~300). Recopying alias pages clobbers GPU-written results not yet copied back. Dropped.
- Found existing fork PRs (dup check): #115 (guard covers every view of pages + title_memory_written for DS; tests) = proper version of our alias+DS fixes; #410 (ahead_asked monotonic in watch::wrote; v3 had same bug); #206 (uncopied pages under pending writes, LATE_WAIT/RELAY, #168). #51 integer RT unblended (refused mesh pipeline; likely blue band at screen bottom = HUD). #24 CBV<=64KiB (other team merged).
- Branch fork-prs = win10-agility + 3e8384a(FULL_REFRESH diag) + stale diag + #115 + #410 + #206 + #51. texture.rs conflict #115 vs #58 resolved (kept both). Tests GPU 117/117. Built in target-prs/ (D3D12 copied). runL.
   - runL CRASH: fork-prs crashed at world entry (frame ~1736) in title code +0x541f03f reading garbage ptr 0x2ee5028a92f => title memory corrupted. Prime suspect #206 (merge of uncopied pages back to title?). runM = fork-prs minus #206 (revert).
   - runM CRASH too (0x80000003 int3 at +0x23d8f03) => not only #206. runN = minus #206 and #115 (keeps #410, #51).
   - runN (fork-prs minus #206 and #115; keeps #410, #51): reaches world, no crash so far, faces clean, ~3 fps (336-394 ms). => #115 causes the crash (int3/AV in title at world entry; likely syscall writes into read-only alias views fail). #206 untested alone. Comment #115 with crash data (ask user first).
- Candidate PRs next: #294/#354/#156 (BVH builder last 3 progs on 32-lane => dynamic shadows), #346 (shader stores by address mark their copy, #284), #363 #240 #255 #384 (copy/copy-back perf), #23 (gameinput history for low fps), #285 (black window before 1st present), #414/#230/#213 (BVH absent child).
   - #115 crash already reported by SantannaCarlos (same int3 +0x23d8f03): DirectStorage kernel read into a destination that #115 made read-only -> ERROR_INVALID_USER_BUFFER 0x800706f8 -> title stops. Also joroyt's fix for #115 protect noting aliases clean: Spexivus/victory-fork#1 (0bcc069) — with it room comes back (their RTX 4070). SantannaCarlos writing hold-open fix for kernel_will_write. Our confirming comment: #115 issuecomment-7148875270. When their fix lands: test #115 + 0bcc069 + hold-open.
   - runN screenshot: 4 fps 270 ms at window; blue band at bottom STILL there with #51 => #51 not the cause of the band.

## jman-picks (branch from fork-prs, 2026-11-25)
- JMAN730/victory = JMAN730/project-flamingo (renamed). Base ffe310e, 471 non-merge commits: 189 apply clean on fork-prs, 247 conflict.
- Picked 131 clean non-doc/CI commits (shader lowering, sRGB/linear views, sampler fixes, null CBV, RTV only for RT textures, A8 typeless, f32_to_half, late flip wait, thin LTO, XAsync/XMA/loader fixes).
- Reverted picks: 9801045 (needs Kept enum), 5f418f2+7183336 (APU tests fail), 1eb3d0a (Win10 token workaround; dup of our #22), e1b8f12 (clzero emulation off by 28 bytes in its test). e4a0f0e, b61b934 did not apply.
- Fixes: Instruction::new (270302d vs branch folding), quota parse type, test call sites.
- lib tests green: shader, formats, hle, pe, loader, pipeline, gpu. ❓ not run in game yet.
- Next candidates (conflict / open PRs): #583 wave64 own-bits (compare with our own_bits.rs), #574 ReBAR upload heaps (copy perf), streamed mips #576, epoch retirement, adapter by SM6.6, issue #331 = Win10 19045 + RTX 5060 report.
- runO (jman-picks, cold pipeline cache: translator changed -> ~1900 pipelines rebuilt, ~55/min at 1-2 fps): ❌ world blurred, bloom blobs, ghosting. Cause: da125a4 (sampler: mip filter none forced MinLOD=MaxLOD=0; old code used min_lod -> title picks a mip of its blur/bloom chain that way). Reverted da125a4.
- runP (jman-picks - da125a4, cache warm from runO): ✅ picture like runN (prison yard, hair, faces OK), no crash. 2.9-3.4 fps, 295-372 ms/frame (prep 106-178, GPU 161-184, copy-back 207-251). 11 untranslated shaders (runN 14). Ceiling panels still float (stale alias copies, unchanged). No fps gain vs runN.
- Rule: a branch that touches shader translation invalidates the pipeline cache; judge pictures only after "(0 being made)".
- Next: diff JMAN PR #583 vs our own_bits.rs (3 BVH programs: divergent readfirstlane); try PR #574 ReBAR upload heaps (copy-back ~220 ms); ceiling = #115 + 0bcc069 + SantannaCarlos hold-open fix when posted.
