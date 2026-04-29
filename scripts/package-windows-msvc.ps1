$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $RootDir "dist/windows"
$PackageRoot = Join-Path $DistDir "aegis-windows-x86_64-msvc"
$Target = "x86_64-pc-windows-msvc"
$ReleaseDir = Join-Path $RootDir "target/$Target/release"
$ZipFile = Join-Path $DistDir "aegis-windows-x86_64-msvc.zip"

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "rustup is required."
}

if (-not (rustup target list --installed | Select-String -SimpleMatch $Target)) {
    throw "Missing Rust target: $Target. Install it with: rustup target add $Target"
}

Remove-Item -Recurse -Force $DistDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $PackageRoot "bin") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $PackageRoot "lib") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $PackageRoot "include") | Out-Null

cargo build -p aegis-ffi --release --target $Target

Copy-Item (Join-Path $RootDir "include/aegis.h") (Join-Path $PackageRoot "include/aegis.h")
Copy-Item (Join-Path $RootDir "include/module.modulemap") (Join-Path $PackageRoot "include/module.modulemap")

$Dll = Join-Path $ReleaseDir "aegis_ffi.dll"
$ImportLib = Join-Path $ReleaseDir "aegis_ffi.dll.lib"
$StaticLib = Join-Path $ReleaseDir "aegis_ffi.lib"
$Pdb = Join-Path $ReleaseDir "aegis_ffi.pdb"

if (-not (Test-Path $Dll)) {
    throw "Missing DLL: $Dll"
}
if (-not (Test-Path $ImportLib)) {
    throw "Missing import library: $ImportLib"
}
if (-not (Test-Path $StaticLib)) {
    throw "Missing static library: $StaticLib"
}

Copy-Item $Dll (Join-Path $PackageRoot "bin/aegis_ffi.dll")
Copy-Item $ImportLib (Join-Path $PackageRoot "lib/aegis_ffi.dll.lib")
Copy-Item $StaticLib (Join-Path $PackageRoot "lib/aegis_ffi.lib")
if (Test-Path $Pdb) {
    Copy-Item $Pdb (Join-Path $PackageRoot "bin/aegis_ffi.pdb")
}

Compress-Archive -Path $PackageRoot -DestinationPath $ZipFile -Force
Write-Output $ZipFile
