# RTX 5060 compute pipeline refusals

## Combined build

On 26 November 2026, the combined build `3adcc40` translated all 25,758 shaders in 83 s with its populated cache. Direct3D 12 created 2,704 compute pipelines and refused 30. The adapter reports 32-lane waves.

The 30 inputs were tested separately from the game using `translate --list --validate --pipelines`. The Windows SDK validator accepted all 30 DXIL modules in both configurations.

| Translator setting | Pipelines created | Wave-size refusals | Other refusals |
| --- | ---: | ---: | ---: |
| Default | 0 | 29 | 1 |
| `VICTORY_WAVE_WIDE=1` | 22 | 7 | 1 |

The combined build used Preload and Play from [PR #47](https://github.com/DexNom/VIctory/pull/47), the compute builder changes from [PR #35](https://github.com/DexNom/VIctory/pull/35), and the POPS translation from [PR #50](https://github.com/DexNom/VIctory/pull/50). In PowerShell, set `VICTORY_FILL_WORKERS`, `VICTORY_PARALLEL_PRELUDES`, and `VICTORY_PIPELINE_WAIT_MS` before invoking the launcher so it inherits the values.

The seven shaders that still require wave64 have thread groups of 128 to 1,024 threads and use local shared memory. The current two-wave fallback declines that multi-wave path. Creation of the other 22 pipelines verifies that the driver accepts their DXIL. The code records previous GPU hangs with some of the broad wave-wide fallbacks, so this experiment was limited to pipeline creation. [Upstream wave32 issue](https://github.com/Marvisak/VIctory/issues/2220)

[PR #40](https://github.com/DexNom/VIctory/pull/40) is related work on an automatic wave-wide fallback. Its report includes two of the shaders in this comparison.

`7f61193d2c0e5f22` repeatedly returns `0x8007000E` on the RTX 5060, including when compiled alone with optimization masks `0`, `1`, `3`, `15`, `31`, and `47`, or with default optimization. The SDK validator accepts its DXIL. The WARP smoke test reports waves of four lanes and stops at the wave-size check before creating a pipeline.

## Upstream comparison

The upstream prototype at `b3efa1c` creates five of the eight pipelines that still fail with `VICTORY_WAVE_WIDE=1` on the RTX 5060:

| Shader hash | Combined build with wave-wide emulation | Upstream prototype |
| --- | --- | --- |
| `2da4beb386509b77` | Requires wave64 | Descriptor tracing refuses this shader |
| `3b02415f67c6b0d8` | Requires wave64 | Pipeline created |
| `1d434307d981352a` | Requires wave64 | Pipeline created |
| `22338086ea4c5364` | Requires wave64 | Pipeline created |
| `02384a60fda6f2d0` | Requires wave64 | Pipeline created |
| `6832a9da15c2e584` | Requires wave64 | Multi-wave lowering still refuses this shader |
| `6b82bc7d22016cb5` | Requires wave64 | Multi-wave lowering still refuses this shader |
| `7f61193d2c0e5f22` | D3D12 returns `0x8007000E` | Pipeline created |

The native metadata for `6832a9da15c2e584` and `6b82bc7d22016cb5` specifies 1,024 threads and 32,768 bytes of shared memory per group. The upstream synchronization state adds 4,480 bytes, bringing both to 37,248 bytes and exceeding the emitter's 32 KiB limit. The four recovered wave64 shaders fit within that limit.

The comparison uses the complete upstream prototype. Its final patch does not apply directly to `3adcc40`, whose emitter has a different layout and synchronization path. The `0x8007000E` result depends on the translator build tested here. The comparison does not identify the exact change that fixes it. [Multi-wave upstream work](https://github.com/Marvisak/VIctory/pull/2214)

## Experimental conversion port

The upstream prototype alone creates 22 pipelines on the 30-shader set. It gains five pipelines over the combined build with wave-wide emulation and loses five that require `v_cvt_f32_f64`:

- `926c46bab3b86bd6`
- `6871d6e4b3ac9a10`
- `5dc3b25986622949`
- `6f225b641c5b0708`
- `a10179ab7eda0dc3`

The isolated worktree `target/wave64-2214-check` ports the existing IEEE 754 double-to-float lowering from the combined build. Its targeted test passed for normal values, rounding ties, subnormals, infinities, NaNs, and input sign modifiers. A rerun of the 30 shaders with just this port still created 22 pipelines. The five listed shaders then reached a missing whole-wave reduction.

Adding the existing double-precision reduction made `5dc3b25986622949` and `6f225b641c5b0708` pass DXIL validation and pipeline creation. The final run created **24 of the 30 pipelines** and reproduced six refusals:

| Shader hash | Refusal in the experimental build |
| --- | --- |
| `2da4beb386509b77` | Descriptor tracing at `0x4d0` |
| `6832a9da15c2e584` | Shared memory budget for the 1,024-thread group |
| `6b82bc7d22016cb5` | Shared memory budget for the 1,024-thread group |
| `926c46bab3b86bd6` | Lane mask used as data at `0x4258` |
| `a10179ab7eda0dc3` | Lane mask used as data at `0x778c` |
| `6871d6e4b3ac9a10` | Lane mask used as data at the image store at `0x4eb4` |

The prototype recovers five pipelines that the combined wave-wide path still refused. That combined path created the three lane-mask pipelines in the table, leaving three shaders that failed in both builds.

The five targeted `console_waves` tests passed in the experimental build, including dispatch checks for several waves per group, divergent branches, loops, and shared memory barriers. The 30 title shaders were evaluated through pipeline creation only. The test launcher and its runtime settings were left as installed.
