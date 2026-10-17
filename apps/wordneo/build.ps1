param(
    [string]$CargoPath = 'cargo',
    [string]$TargetDirectory = '',
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$cargo = (Get-Command $CargoPath -ErrorAction Stop).Source
$env:PATH = (Split-Path $cargo -Parent) + [IO.Path]::PathSeparator + $env:PATH
$rustc = if ($env:RUSTC) { $env:RUSTC } else { (Get-Command rustc -ErrorAction Stop).Source }
$env:RUSTC = $rustc
$hostLine = & $rustc -vV | Where-Object { $_ -like 'host: *' }
if ($LASTEXITCODE -ne 0 -or -not $hostLine) { throw 'Cannot identify Rust toolchain' }
$hostTriple = $hostLine.Substring(6).Trim()
$sysroot = (& $rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate Rust sysroot' }
if ($TargetDirectory) { $env:CARGO_TARGET_DIR = [IO.Path]::GetFullPath($TargetDirectory) }
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $root 'target' }
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_RELEASE_LTO = 'off'
$env:CARGO_PROFILE_RELEASE_OPT_LEVEL = '2'
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = '16'
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '4' }
if ($hostTriple -eq 'x86_64-pc-windows-gnu') {
    $nativeBin = Join-Path $sysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
    if (Test-Path -LiteralPath $nativeBin) { $env:PATH = $nativeBin + [IO.Path]::PathSeparator + $env:PATH }
    $dlltool = Get-Command dlltool.exe,x86_64-w64-mingw32-dlltool.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $dlltool) { throw 'GNU builds require MinGW-w64 dlltool on PATH; use the MSVC toolchain or provide the GNU tools' }
    $imports = Join-Path $root '.build\native-imports'
    New-Item -ItemType Directory -Path $imports -Force | Out-Null
    & $dlltool.Source -d (Join-Path $root 'native\shlwapi.def') -l (Join-Path $imports 'libshlwapi.a')
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create the Windows import library' }
    $env:WORDNEO_NATIVE_LIB_DIR = $imports
    $gcc = Get-Command x86_64-w64-mingw32-gcc.exe -ErrorAction SilentlyContinue
    if ($gcc) { $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $gcc.Source }
}
$arguments = @('build', '--locked', '--release', '-p', 'wordcraft', '-p', 'wordcraft-cli')
if ($Offline) { $arguments += '--offline' }
Push-Location -LiteralPath $root
try {
    & $cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw "Cargo failed: $LASTEXITCODE" }
    $release = Join-Path $env:CARGO_TARGET_DIR 'release'
    $dist = Join-Path $root 'dist\wordneo-012'
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $release 'wordcraft.exe') -Destination (Join-Path $dist 'WordNeo.exe')
    Copy-Item -LiteralPath (Join-Path $release 'wordcraft-cli.exe') -Destination (Join-Path $dist 'WordNeo-cli.exe')
    foreach ($file in @('LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'ATTRIBUTION.md')) {
        Copy-Item -LiteralPath (Join-Path $root $file) -Destination $dist
    }
    Get-ChildItem -LiteralPath (Join-Path $root 'assets\fonts') -Filter 'OFL-*.txt' | Copy-Item -Destination $dist
    foreach ($file in @('Iniciar-WordNeo.ps1', 'private-data.ps1', 'USO.md')) {
        Copy-Item -LiteralPath (Join-Path $root ('portable\' + $file)) -Destination $dist
    }
    New-Item -ItemType Directory -Path (Join-Path $dist 'data') -Force | Out-Null
    Write-Output "WordNeo 012: $dist"
} finally { Pop-Location }
