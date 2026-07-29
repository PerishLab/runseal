$ErrorActionPreference = 'Stop'

$command = if ($args.Length -gt 0) { $args[0] } else { 'install' }
$rest = if ($args.Length -gt 1) { $args[1..($args.Length - 1)] } else { @() }

$channel = if ($env:RUNSEAL_CHANNEL) { $env:RUNSEAL_CHANNEL } else { 'stable' }
$version = if ($env:RUNSEAL_VERSION) { $env:RUNSEAL_VERSION } else { '' }
$publicUrl = if ($env:RUNSEAL_RELEASES_PUBLIC_URL) { $env:RUNSEAL_RELEASES_PUBLIC_URL } else { 'https://releases.runseal.perish.uk' }
$installRoot = if ($env:RUNSEAL_INSTALL_ROOT) { $env:RUNSEAL_INSTALL_ROOT } else { Join-Path $HOME '.local/share/runseal' }
$localBinDir = if ($env:RUNSEAL_LOCAL_BIN_DIR) { $env:RUNSEAL_LOCAL_BIN_DIR } else { Join-Path $HOME '.local/bin' }

for ($i = 0; $i -lt $rest.Length; $i++) {
    switch -Regex ($rest[$i]) {
        '^--channel$' { $i++; $channel = $rest[$i]; continue }
        '^--channel=(.+)$' { $channel = $Matches[1]; continue }
        '^--version$' { $i++; $version = $rest[$i]; continue }
        '^--version=(.+)$' { $version = $Matches[1]; continue }
        '^--public-url$' { $i++; $publicUrl = $rest[$i]; continue }
        '^--public-url=(.+)$' { $publicUrl = $Matches[1]; continue }
        '^--install-root$' { $i++; $installRoot = $rest[$i]; continue }
        '^--install-root=(.+)$' { $installRoot = $Matches[1]; continue }
        '^--bin-dir$' { $i++; $localBinDir = $rest[$i]; continue }
        '^--bin-dir=(.+)$' { $localBinDir = $Matches[1]; continue }
        '^-h$|^--help$|^help$' {
            @'
runseal manager

Usage:
  manage.ps1 install [--channel stable|beta] [--version vX.Y.Z] [--public-url <url>]
  manage.ps1 update  [--channel stable|beta] [--version vX.Y.Z] [--public-url <url>]
  manage.ps1 uninstall [--version vX.Y.Z]
'@ | Write-Output
            exit 0
        }
        default { throw "unknown argument: $($rest[$i])" }
    }
}

function Need-PublicUrl {
    if ([string]::IsNullOrWhiteSpace($publicUrl)) {
        throw 'RUNSEAL_RELEASES_PUBLIC_URL or --public-url is required'
    }
    return $publicUrl.TrimEnd('/')
}

function Latest-Version($metadataPath) {
    $metadata = Get-Content -Raw -Path $metadataPath | ConvertFrom-Json
    return $metadata.releaseVersion
}

function Install-Runseal {
    $baseUrl = Need-PublicUrl
    $tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("runseal-install-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmpdir | Out-Null
    try {
        if ([string]::IsNullOrWhiteSpace($version)) {
            $metadataPath = Join-Path $tmpdir 'metadata.json'
            Invoke-WebRequest -Uri "$baseUrl/$channel/latest/metadata.json" -OutFile $metadataPath
            $script:version = Latest-Version $metadataPath
            if ([string]::IsNullOrWhiteSpace($script:version)) {
                throw 'failed to resolve latest runseal version'
            }
        }

        $archive = 'runseal-x86_64-pc-windows-msvc.zip'
        $archivePath = Join-Path $tmpdir $archive
        Invoke-WebRequest -Uri "$baseUrl/$channel/versions/$version/$archive" -OutFile $archivePath

        $versionRoot = Join-Path $installRoot $version
        New-Item -ItemType Directory -Force -Path $versionRoot | Out-Null
        New-Item -ItemType Directory -Force -Path $localBinDir | Out-Null
        Expand-Archive -LiteralPath $archivePath -DestinationPath $versionRoot -Force

        $cmd = Join-Path $localBinDir 'runseal.cmd'
        $exe = Join-Path $versionRoot 'runseal.exe'
        "@echo off`r`n`"$exe`" %*`r`n" | Set-Content -Encoding ASCII -Path $cmd
        & $cmd --version
        Write-Output "installed runseal to $cmd"
        $swept = Get-ChildItem -Directory -ErrorAction SilentlyContinue $installRoot |
            Where-Object { $_.Name -ne $version }
        foreach ($seat in $swept) {
            Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $seat.FullName
        }
        if ($swept) {
            Write-Output "swept: $($swept.Name -join ' ')"
        }
    }
    finally {
        Remove-Item -LiteralPath $tmpdir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Uninstall-Runseal {
    $cmd = Join-Path $localBinDir 'runseal.cmd'
    Remove-Item -LiteralPath $cmd -Force -ErrorAction SilentlyContinue
    Write-Output "removed $cmd"

    if ([string]::IsNullOrWhiteSpace($version)) {
        Remove-Item -LiteralPath $installRoot -Recurse -Force -ErrorAction SilentlyContinue
        Write-Output "removed $installRoot"
    }
    else {
        $versionRoot = Join-Path $installRoot $version
        Remove-Item -LiteralPath $versionRoot -Recurse -Force -ErrorAction SilentlyContinue
        Write-Output "removed $versionRoot"
    }
}

switch ($command) {
    'install' { Install-Runseal }
    'update' { Install-Runseal }
    'uninstall' { Uninstall-Runseal }
    default { throw "unknown command: $command" }
}
