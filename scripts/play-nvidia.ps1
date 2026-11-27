# Starts the title on an NVIDIA card with the verified nvidia-hybrid-v2 Series S settings
# (Pass 49: locked 30.0 fps real-time pacing at 17.7-20.9 ms CPU prep, 36-47 fps uncapped on
# RTX 4080 / i5-13600K; 0 device losses, transparent glass/windows, haze-free interiors, trees fixed).
#
#   scripts\play-nvidia.ps1 -Exe <path of your own copy's executable>
#   scripts\play-nvidia.ps1 -Exe <path> -Warm   # precompile all shaders + used D3D12 PSOs with matching cache keys
#
# -Warm            Precompile shaders (`victory shaders`) and used D3D12 PSOs (`victory warm --used`) with the
#                  exact same translator switches before running, then exit. Eliminates intro/world-entry compile stalls.
# -TargetFps       Override the title's 30 fps clock (e.g. '60', '120', 'uncapped'). Default: unset (locked 30.000 fps
#                  with precise hybrid sleep+spin pacer for exact game-logic timing and clean Smooth Motion / LSFG).
# -NoSmoothMotion  Run target\<build>\victory.exe itself. By default a copy named victory_sm.exe is started instead,
#                  so an NVIDIA Smooth Motion profile set for that exe in the NVIDIA App applies every time (30 -> 60 fps).
# -SeriesX         Use the Series X profile instead (slower, 2160p output).
# -Quality         Stricter relay/step mode (VICTORY_RELAY=1, VICTORY_CARD_STEPS=0) if you still see mesh glitches.
# -RayTracing      Keep the title's ray-tracing passes (off by default: see rt_off_hashes.txt + rt_work_off_hashes.txt).
# -Resolution      Internal render height before the title's own upscaler: 2160, 1440, 1080, 720, 540 or 480.
#                  Series S outputs 1080p (default 540p = 0.5 scale); Series X outputs 2160p (default 1080p = 0.5 scale).
# -Adapter         Pick a GPU on multi-GPU PCs: part of its name (-Adapter 5060) or index (-Adapter '#1').
# -Build           target\release (default; skip list & mimalloc work in release on hybrid-v2) or target\diag.
# -More            Extra overrides: -More @{VICTORY_STREAM_REFRESH='0'}
#
# Notes for specific hardware:
# - ReBAR BAR caps (VICTORY_SMALL_BUFFERS_BAR_MB / VICTORY_WINDOWS_BAR_MB) are NOT set by default: they render
#   3D black when ReBAR is enabled (fxceless/victory-fork#1268).
# - VICTORY_STREAM_REFRESH is left at its default (CPU-dependent: faster on Ryzen 5800X, pass
#   -More @{VICTORY_STREAM_REFRESH='0';VICTORY_STREAM_COPY='0'} on Intel 13600K).
# - Windows 10: place Agility SDK D3D12\D3D12Core.dll beside victory.exe (or set VICTORY_D3D12_AGILITY).
#   Windows 11: loads D3D12\D3D12Core.dll if present, else falls back cleanly to system d3d12.dll (no 0x887E0003).
# Run from a normal (non-admin) window so the driver uses your normal shader cache.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Warm,
    [string]$TargetFps = '',
    [switch]$NoSmoothMotion,
    [switch]$SeriesX,
    [switch]$Quality,
    [switch]$RayTracing,
    [ValidateSet('2160', '1440', '1080', '720', '540', '480')][string]$Resolution,
    [string]$Adapter,
    [ValidateSet('release', 'diag')][string]$Build = 'release',
    [hashtable]$More = @{},
    [string]$VictoryExe,
    [string]$Label
)
$ErrorActionPreference = 'Stop'

$settings = [ordered]@{
    # Core threading, relay, copy-back, barriers & descriptor sync (Pass 47-49 verified)
    VICTORY_RELAY                  = '2'
    VICTORY_CARD_STEPS             = '1'
    VICTORY_TEXTURE_NO_WAIT        = '0'
    VICTORY_WIDE_BACK              = '1'
    VICTORY_CARD_REHASH_RUNS       = '1'
    VICTORY_JOIN_COPIES            = '1'
    VICTORY_INDIRECT_WINDOW        = '1'
    VICTORY_RELAY_COPY             = '1'
    VICTORY_INDIRECT_BATCH         = '2'
    VICTORY_MAP_THREADS            = '1'
    VICTORY_TITLE_PRIORITIES       = '0'
    VICTORY_AFFINITY               = 'free'
    VICTORY_SHOW_AFTER             = '1'
    VICTORY_D3D12_DRED             = 'light'
    VICTORY_THIN                   = '1'
    VICTORY_JOIN_PARTS             = '16'
    VICTORY_JOIN_LISTS             = '96'
    VICTORY_UPLOAD_AHEAD           = '1'
    VICTORY_LAND_ASIDE             = '1'
    VICTORY_LAND_SKIP              = '1'
    VICTORY_LAND_THREADS           = '8'
    VICTORY_VIEW_THREADS           = '8'
    VICTORY_SKIP_WC_BACK           = '1'
    VICTORY_SKIP_WC_COMPARE        = '1'
    VICTORY_SMART_BARRIERS         = '1'
    VICTORY_DEDUP_BARRIERS         = '1'
    VICTORY_COPY_BESIDE            = '1'
    VICTORY_STORE_BESIDE           = '1'
    VICTORY_MAP_LOCK_FINE          = '1'
    VICTORY_PARTIAL_DELTA          = '1'
    VICTORY_BAKE_DELTA             = '1'
    VICTORY_ASK_TEXTURES           = '1'
    VICTORY_REFRESH_ONCE           = '1'
    VICTORY_WINDOW_REUSE           = '1'
    VICTORY_LAZY_COPYBACK          = '1'
    VICTORY_UP_QUEUE               = '1'
    VICTORY_DISK_CORES             = 'adaptive'
    VICTORY_CARD_AWAKE             = '0'
    VICTORY_WIDE_SPIN              = '10'
    VICTORY_LANE_CORE              = '0'
    VICTORY_UM_COALESCE_KB         = '0'
    VICTORY_SAMPLERS_MADE          = '0'
    VICTORY_PORT_JOURNAL           = '0'
    VICTORY_KEEP_PAGES             = '0'
    VICTORY_PRECISE_PACE           = '1'
    # Depth-plane & descriptor alias correctness + E5 render-target stand-in
    VICTORY_PLANE_NOW              = '1'
    VICTORY_LIST_ALIAS_EXACT       = '1'
    VICTORY_E5_TARGETS             = 'r11g11b10'
    # Card buffers (64 KiB threshold + 2048 MiB pool + fetch check)
    VICTORY_CARD_BUFFERS_MIN_KB    = '64'
    VICTORY_CARD_BUFFERS_MB        = '2048'
    VICTORY_FETCH_CHECK            = '1'
    # Streaming, bakes, and background pipeline compilation + book recording for `victory warm --used`
    VICTORY_SECTION_MB             = '2'
    VICTORY_TEXTURE_STAGE_MB       = '768'
    VICTORY_TEXTURE_STAGE_MS       = '50'
    VICTORY_TEXTURE_CATCHUP        = '1'
    VICTORY_BAKE_TEXTURES_MS       = '1500'
    VICTORY_PIPELINE_WORKERS       = '6'
    VICTORY_PIPELINE_LOWEST        = '1'
    VICTORY_PIPELINE_WAIT_MS       = '500'
    VICTORY_PIPELINE_BOOK          = '1'
    # Indirect light, interior haze clamp, transparent glass/windows & pure-clamp specular stand-in
    VICTORY_AMBIENT                = '0.75'
    VICTORY_AMBIENT_AIR            = '1.5'
    VICTORY_AMBIENT_MOST           = '2'
    VICTORY_AMBIENT_LEFTOVER       = '0'
    VICTORY_AMBIENT_SHADE          = '1.1,0.6'
    VICTORY_AMBIENT_REFLECT        = '0.65'
    VICTORY_AMBIENT_SETTLE         = '250,3'
    VICTORY_AMBIENT_MARCH          = '3'
    VICTORY_AMBIENT_SPECULAR       = '0.8,0.1'
    VICTORY_AMBIENT_SPECULAR_ALPHA = 'clamp'
    # Shader translator NVIDIA switches (cached by key; matched automatically by -Warm)
    VICTORY_NAN_SCRUB              = '1'
    VICTORY_BVH_MISS               = '1'
    VICTORY_NV_FLOAT               = '7'
    VICTORY_SPLIT_LDS_COALESCE     = '1'
    VICTORY_LDS_WRITE_ONLY_OK      = '1'
    # NVIDIA view-heap cap + Series S half scale, own upscaler, second interface plane, watchdog disarm
    VICTORY_DYNAMIC_SLOTS          = '524288'
    VICTORY_RENDER_SCALE           = '0.5'
    VICTORY_OWN_UPSCALER           = '1'
    VICTORY_COMPOSE                = '1'
    VICTORY_NO_WATCHDOG            = '1'
    VICTORY_SKIP_PIPELINES         = 'none'
    VICTORY_SKIP_SYNC              = '0'
}

# Skip 1,034 RT resolve/shadow shaders + 43 RT-only bvh_skinning/Primary_GatherCopy shaders when RT is off.
$rtList = Join-Path $PSScriptRoot 'rt_off_hashes.txt'
$rtWork = Join-Path $PSScriptRoot 'rt_work_off_hashes.txt'
if (-not $RayTracing -and (Test-Path $rtList)) {
    $hashes = (Get-Content $rtList -Raw).Trim()
    if (Test-Path $rtWork) {
        $extra = (Get-Content $rtWork -Raw).Trim()
        if ($extra -ne '') { $hashes = "$hashes,$extra" }
    }
    $settings.VICTORY_SKIP_PIPELINES = $hashes
    $settings.VICTORY_SKIP_SYNC = '1'
}

if ($TargetFps -ne '') {
    $settings.VICTORY_TARGET_FPS = $TargetFps
}

if ($Quality) {
    $settings.VICTORY_RELAY = '1'
    $settings.VICTORY_CARD_STEPS = '0'
    $settings.VICTORY_TEXTURE_NO_WAIT = '0'
}

$outHeight = if ($SeriesX) { 2160 } else { 1080 }
$console = if ($SeriesX) { 'X' } else { 'S' }
if ($Resolution) {
    if ([int]$Resolution -gt $outHeight) {
        Write-Host "Series $console outputs ${outHeight}p: ${Resolution}p is above it."
        return
    }
    $settings.VICTORY_RENDER_SCALE = ([int]$Resolution / $outHeight).ToString('0.####', [Globalization.CultureInfo]::InvariantCulture)
}
if ($Adapter) { $settings.VICTORY_ADAPTER = $Adapter }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }
if ($Label) { $settings.VICTORY_RUN_LABEL = $Label }   # shown in the window text

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if ($VictoryExe) { $victory = $VictoryExe }   # another build (bisecting)
if (-not (Test-Path $victory)) {
    Write-Host "No $victory. Build it first: cargo build --release -p victory"
    return
}
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; return }

# Clean stale VICTORY_* vars while preserving user path configuration.
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR', 'VICTORY_D3D12_AGILITY'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }

$Exe = (Resolve-Path $Exe).Path
$gameDir = Split-Path $Exe
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try {
    if ($Warm) {
        $shaderRpf = Join-Path $gameDir 'shaders.rpf'
        $target = if (Test-Path $shaderRpf) { $shaderRpf } else { $gameDir }
        Write-Host "Warming DXIL shader cache from $target with matching translator switches..."
        & $victory shaders $target
        Write-Host "Warming used D3D12 PSOs (victory warm --used)..."
        & $victory warm --used
        return
    }
    $scale = [double]::Parse($settings.VICTORY_RENDER_SCALE, [Globalization.CultureInfo]::InvariantCulture)
    $pace = if ($settings.Contains('VICTORY_TARGET_FPS')) { "target=$($settings.VICTORY_TARGET_FPS) fps" } else { 'locked 30 fps (precise pacer)' }
    Write-Host "Series $console, rendering at $([int]($outHeight * $scale))p (scale $($settings.VICTORY_RENDER_SCALE)), $pace, build=$Build"
    if (-not $NoSmoothMotion) {
        $sm = Join-Path (Split-Path $victory) 'victory_sm.exe'
        if (-not (Test-Path $sm) -or (Get-Item $sm).LastWriteTime -lt (Get-Item $victory).LastWriteTime) {
            Copy-Item $victory $sm -Force
        }
        $victory = $sm
    }
    $runArgs = @('run', $Exe, '--execute')
    if (-not $SeriesX) { $runArgs += @('--console', 'series-s') }
    & $victory @runArgs
} finally {
    Pop-Location
}
