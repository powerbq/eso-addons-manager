[Console]::OutputEncoding = [Text.Encoding]::UTF8
Set-Location -ErrorAction Stop (Split-Path -Parent $PSScriptRoot)

$metadata = cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$package = ($metadata | ConvertFrom-Json).packages | Where-Object { $_.name -eq 'litehtml-sys' }

$patch = Join-Path $PSScriptRoot "litehtml-sys-$($package.version)-msvc.patch"
if (-not (Test-Path $patch)) { throw "No patch for litehtml-sys $($package.version)" }

$patched = 'target/patched/litehtml-sys'
if (Test-Path $patched) { Remove-Item -ErrorAction Stop -Recurse -Force $patched }
New-Item -ErrorAction Stop -ItemType Directory -Force (Split-Path $patched) | Out-Null
Copy-Item -ErrorAction Stop -Recurse (Split-Path -Parent $package.manifest_path) $patched

git apply "--directory=$patched" $patch
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$path = (Resolve-Path -ErrorAction Stop $patched).Path -replace '\\', '/'
$lock = [IO.File]::ReadAllBytes("$PWD/Cargo.lock")
try {
    cargo --config "patch.crates-io.litehtml-sys.path='$path'" @args
} finally {
    [IO.File]::WriteAllBytes("$PWD/Cargo.lock", $lock)
}
exit $LASTEXITCODE
