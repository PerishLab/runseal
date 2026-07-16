$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$appDir = Join-Path $root 'app'
$name = 'runseal'
$cargoToml = Join-Path $appDir 'Cargo.toml'
$version = (Select-String -Path $cargoToml -Pattern '^version = "(.+)"$').Matches[0].Groups[1].Value
$releaseVersion = if ($args.Length -gt 0 -and -not [string]::IsNullOrWhiteSpace($args[0])) { $args[0] } elseif ($env:RELEASE_VERSION) { $env:RELEASE_VERSION } else { "v$version" }
$target = if ($env:TARGET) { $env:TARGET } else { 'x86_64-pc-windows-msvc' }
$distDir = if ($env:DIST_DIR) { $env:DIST_DIR } else { Join-Path $root 'dist' }
$artifactDir = Join-Path $distDir $releaseVersion

New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
$env:RUNSEAL_BUILD_VERSION = $releaseVersion
$env:RUNSEAL_BUILD_CHANNEL = if ($env:RELEASE_CHANNEL) { $env:RELEASE_CHANNEL } else { 'dev' }
$env:RUNSEAL_BUILD_PUBLIC_URL = if ($env:RUNSEAL_RELEASES_PUBLIC_URL) { $env:RUNSEAL_RELEASES_PUBLIC_URL } else { '' }
cargo build --release --locked -p runseal --target $target

$archive = "$name-$target.zip"
$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("$name-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null

try {
    $bin = Join-Path $root "target/$target/release/$name.exe"
    Copy-Item $bin (Join-Path $tmpdir "$name.exe")
    Compress-Archive -LiteralPath (Join-Path $tmpdir "$name.exe") -DestinationPath (Join-Path $artifactDir $archive) -Force
    Write-Output (Join-Path $artifactDir $archive)
}
finally {
    Remove-Item -LiteralPath $tmpdir -Recurse -Force -ErrorAction SilentlyContinue
}
