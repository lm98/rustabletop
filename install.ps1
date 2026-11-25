$ErrorActionPreference = 'Stop'

function Install-Rustabletop {
    $repo = 'lm98/rustabletop'
    $fallback = "cargo install --git https://github.com/$repo rustabletop"
    $binDir = if ($env:RUSTABLETOP_INSTALL_DIR) { $env:RUSTABLETOP_INSTALL_DIR } else { Join-Path $HOME '.local\bin' }

    function Fail($message) {
        Write-Host "error: $message" -ForegroundColor Red
        Write-Host "fallback (builds from source, ~8 minutes): $fallback"
        throw $message
    }

    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    if ($arch -ne 'AMD64') { Fail "no prebuilt rustabletop for Windows $arch" }
    $target = 'x86_64-pc-windows-msvc'

    $tag = $env:RUSTABLETOP_VERSION
    if (-not $tag) {
        try {
            $releases = Invoke-RestMethod -UseBasicParsing "https://api.github.com/repos/$repo/releases?per_page=1"
            $tag = @($releases)[0].tag_name
        } catch { $tag = $null }
        if (-not $tag) { Fail "no release of $repo found" }
    }

    $url = "https://github.com/$repo/releases/download/$tag/rustabletop-$target.zip"
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rustabletop-" + [System.Guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $zip = Join-Path $tmp 'rustabletop.zip'
        try { Invoke-WebRequest -UseBasicParsing $url -OutFile $zip } catch { Fail "cannot download $url" }
        Expand-Archive -Path $zip -DestinationPath $tmp -Force
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
        Copy-Item (Join-Path $tmp 'rustabletop.exe') (Join-Path $binDir 'rustabletop.exe') -Force
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }

    Write-Host "installed rustabletop $tag in $binDir"
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ })
    if ($entries -notcontains $binDir) {
        if ($env:RUSTABLETOP_NO_MODIFY_PATH) {
            Write-Host "add it to PATH: $binDir"
        } else {
            [Environment]::SetEnvironmentVariable('Path', (($entries + $binDir) -join ';'), 'User')
            Write-Host "added $binDir to the user PATH (open a new terminal to use it)"
        }
    }
    if (($env:Path -split ';') -notcontains $binDir) { $env:Path = "$binDir;$env:Path" }
    & (Join-Path $binDir 'rustabletop.exe') --version
}

Install-Rustabletop
