param(
    [string]$Version = "",
    [string]$InstallDir = "",
    [switch]$Force,
    [switch]$DryRun,
    [switch]$NoModifyPath,
    [switch]$NoDeps,
    [switch]$Uninstall,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {}
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls13
} catch {}
Set-StrictMode -Version Latest

$AppName = "SumanMovies-Tui"
$BinName = "sumanmovies.exe"
$Repo = "SumanCH8514/SumanMovies-Tui"
$DefaultInstallDir = Join-Path $env:LOCALAPPDATA "Programs\SumanMovies-Tui\bin"

if ($Help) {
    Write-Host @"
SumanMovies-TUI Installer (Windows PowerShell)

USAGE:
    irm https://raw.githubusercontent.com/SumanCH8514/SumanMovies-Tui/main/install.ps1 | iex
    .\install.ps1 [OPTIONS]

OPTIONS:
    -Version <tag>       Install a specific version (e.g. v0.1.14)
    -InstallDir <path>   Install binary to a custom directory
    -Force               Reinstall even if already at the latest version
    -DryRun              Perform preflight checks without writing files
    -NoModifyPath        Do not modify User PATH environment variable
    -NoDeps              Skip automatic prerequisite installation (mpv, yt-dlp, ffmpeg)
    -Uninstall           Uninstall SumanMovies-TUI from your system
    -Help                Show this help message
"@
    exit 0
}

function Write-Step { param([string]$Message) Write-Host "  > " -ForegroundColor Cyan -NoNewline; Write-Host $Message }
function Write-Success { param([string]$Message) Write-Host "  + " -ForegroundColor Green -NoNewline; Write-Host $Message }
function Write-Warn { param([string]$Message) Write-Host "  ! " -ForegroundColor Yellow -NoNewline; Write-Host $Message }
function Write-Err { param([string]$Message) Write-Host "  x " -ForegroundColor Red -NoNewline; Write-Host $Message; exit 1 }

if ($Version -and $Version[0] -ne "v") {
    $Version = "v$Version"
}

function Get-UserPathRaw {
    param([ref]$Kind)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment")
    if (-not $key) { return "" }
    try {
        $raw = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        try { $Kind.Value = $key.GetValueKind("Path") } catch { $Kind.Value = [Microsoft.Win32.RegistryValueKind]::ExpandString }
        return [string]$raw
    } finally {
        $key.Close()
    }
}

function Set-UserPathRaw {
    param([string]$NewPath, [Microsoft.Win32.RegistryValueKind]$ValueKind)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
    if (-not $key) { return }
    try { $key.SetValue("Path", $NewPath, $ValueKind) } finally { $key.Close() }
}

function Add-ToUserPath {
    param([string]$Directory)
    $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    $raw = Get-UserPathRaw -Kind ([ref]$kind)
    $segments = @($raw -split ";" | Where-Object { $_ })
    if ($segments -notcontains $Directory.TrimEnd("\")) {
        $joined = if ($raw.Trim()) { "$raw;$Directory" } else { $Directory }
        Set-UserPathRaw -NewPath $joined -ValueKind $kind
        return $true
    }
    return $false
}

function Remove-FromUserPath {
    param([string[]]$Directories)
    $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    $raw = Get-UserPathRaw -Kind ([ref]$kind)
    if (-not $raw) { return $false }
    $normalized = @($Directories | ForEach-Object { $_.TrimEnd("\").ToLowerInvariant() })
    $kept = @($raw -split ";" | Where-Object { $_ -and ($normalized -notcontains $_.TrimEnd("\").ToLowerInvariant()) })
    if ($kept.Count -eq @($raw -split ";" | Where-Object { $_ }).Count) { return $false }
    Set-UserPathRaw -NewPath ($kept -join ";") -ValueKind $kind
    return $true
}

function Get-TerminalCols {
    $Cols = 80
    try {
        if ($Host.UI.RawUI.WindowSize.Width -gt 0) {
            $Cols = $Host.UI.RawUI.WindowSize.Width
        }
    } catch {
        $Cols = 80
    }
    return $Cols
}

function Print-Header {
    try { [Console]::Clear() } catch { Clear-Host }
    $Cols = Get-TerminalCols

    if ($Cols -ge 65) {
        $BannerWidth = 63
        $Lines = @(
            " ____                          __  __            _           ",
            "/ ___|_   _ _ __ ___   __ _ _ _|  \/  | _____   _(_) ___  ___",
            "\___ \ | | | '_ `` _ \ / _`` | '_ \ |\/| |/ _ \ \ / / |/ _ \/ __|",
            " ___) | |_| | | | | | | (_| | | | |  | | (_) \ V /| |  __/\__ \",
            "|____/ \__,_|_| |_| |_|\__,_|_| |_|  |_|\___/ \_/ |_|\___||___/"
        )
    } elseif ($Cols -ge 36) {
        $BannerWidth = 34
        $Lines = @(
            "==================================",
            "        SUMANMOVIES TUI           ",
            "=================================="
        )
    } else {
        $BannerWidth = 15
        $Lines = @(
            "SumanMovies-TUI"
        )
    }

    $BannerPad = [Math]::Max(0, [int][Math]::Floor(($Cols - $BannerWidth) / 2))
    $Sub = "SumanMovies Official Installer"
    $SubPad = [Math]::Max(0, [int][Math]::Floor(($Cols - $Sub.Length) / 2))

    foreach ($Line in $Lines) {
        if ($BannerPad -gt 0) {
            Write-Host ((" " * $BannerPad) + $Line) -ForegroundColor Magenta
        } else {
            Write-Host $Line -ForegroundColor Magenta
        }
    }

    if ($SubPad -gt 0) {
        Write-Host ((" " * $SubPad) + $Sub + "`n") -ForegroundColor Cyan
    } else {
        Write-Host ($Sub + "`n") -ForegroundColor Cyan
    }
}

function Do-Uninstall {
    Print-Header
    Write-Step "Uninstalling $AppName..."
    
    $Found = $false
    $TargetDirs = @(
        $DefaultInstallDir,
        "$env:LOCALAPPDATA\SumanMovies-Tui"
    )

    foreach ($Dir in $TargetDirs) {
        foreach ($Name in @($BinName, "sumanmovies-tui.exe")) {
            $Exe = Join-Path $Dir $Name
            if (Test-Path $Exe) {
                try {
                    $RunningProcesses = Get-Process -Name "sumanmovies-tui","sumanmovies" -ErrorAction SilentlyContinue
                    if ($RunningProcesses) {
                        $RunningProcesses | Stop-Process -Force
                        Start-Sleep -Seconds 1
                    }
                    Remove-Item -Path $Exe -Force -ErrorAction SilentlyContinue
                    Write-Success "Removed $Exe"
                    $Found = $true
                } catch {
                    Write-Warn "Could not remove $Exe"
                }
            }
        }
    }

    if ($Found) {
        $Removed = Remove-FromUserPath -Directories @($DefaultInstallDir, "$env:LOCALAPPDATA\SumanMovies-Tui\bin")
        Write-Success "$AppName was successfully uninstalled."
        if ($Removed) {
            Write-Success "Removed stale entry from User PATH."
        }
    } else {
        Write-Warn "No installed binary of $BinName was found."
    }
    exit 0
}

if ($Uninstall) {
    Do-Uninstall
}

Print-Header

$Architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($Architecture -eq "ARM64") {
    $ArchiveName = "SumanMovies_Windows_arm64.zip"
    $PlatformName = "Windows (arm64)"
} elseif ($Architecture -eq "AMD64") {
    $ArchiveName = "SumanMovies_Windows_x64.zip"
    $PlatformName = "Windows (x64)"
} else {
    Write-Err "Unsupported Windows architecture: $Architecture"
}

Write-Step "[1/5] Checking environment & resolving version..."

$TargetVersion = $Version
if (-not $TargetVersion) {
    try {
        $Request = [System.Net.WebRequest]::Create("https://github.com/$Repo/releases/latest")
        $Request.AllowAutoRedirect = $false
        $Response = $Request.GetResponse()
        $Location = $Response.Headers["Location"]
        if ($Location) {
            $TargetVersion = $Location.Split("/")[-1].Trim()
        }
        $Response.Close()
    } catch {
        try {
            $ReleaseJson = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "SumanMovies-Installer" } -UseBasicParsing
            $TargetVersion = $ReleaseJson.tag_name.Trim()
        } catch {
            Write-Err "Failed to contact GitHub for latest release. Please check your internet connection."
        }
    }
}

if (-not $TargetVersion) {
    Write-Err "Could not resolve latest release version from GitHub."
}

Write-Success "[1/5] Environment ready ($PlatformName - $TargetVersion)"

$EffectiveInstallDir = if ($InstallDir) { $InstallDir } else { $DefaultInstallDir }
$ExePath = Join-Path $EffectiveInstallDir $BinName

if (Test-Path $ExePath) {
    try {
        $CurrentVerOutput = (& $ExePath --version 2>&1 | Out-String)
        if ($CurrentVerOutput -match "sumanmovies-tui\s+([\d\.]+)") {
            $CurrentVer = "v" + $matches[1]
            if ($CurrentVer -eq $TargetVersion -and (-not $Force)) {
                Write-Success "SumanMovies-TUI $TargetVersion is already installed at $ExePath. Use -Force to reinstall."
                exit 0
            }
        }
    } catch {}
}

if ($DryRun) {
    Write-Success "[Dry Run] Target package: $ArchiveName"
    Write-Success "[Dry Run] Target install directory: $ExePath"
    Write-Success "[Dry Run] All preflight checks passed."
    exit 0
}

function Install-Prerequisites {
    param([string]$TargetBinDir)

    if ($NoDeps) {
        Write-Step "Skipping prerequisite configuration (-NoDeps specified)."
        return
    }

    Write-Step "[2/5] Configuring prerequisites (mpv, yt-dlp, ffmpeg)..."

    # 1. mpv media player
    $HasMpv = (Get-Command "mpv" -ErrorAction SilentlyContinue) -or `
              (Test-Path "C:\Program Files\mpv\mpv.exe") -or `
              (Test-Path "C:\Program Files\MPV Player\mpv.exe") -or `
              (Test-Path "C:\mpv\mpv.exe") -or `
              (Test-Path "$env:LOCALAPPDATA\Programs\mpv\mpv.exe")

    if ($HasMpv) {
        Write-Success "mpv player is already installed"
    } else {
        $InstalledMpv = $false
        if (Get-Command "winget" -ErrorAction SilentlyContinue) {
            Write-Step "Installing mpv via winget..."
            try {
                $p = Start-Process -FilePath "winget" -ArgumentList "install --id io.mpv --source winget --accept-source-agreements --accept-package-agreements --silent" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed mpv player via winget"
                    $InstalledMpv = $true
                }
            } catch {}
        }
        if (-not $InstalledMpv -and (Get-Command "scoop" -ErrorAction SilentlyContinue)) {
            Write-Step "Installing mpv via scoop..."
            try {
                $p = Start-Process -FilePath "scoop" -ArgumentList "install mpv" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed mpv player via scoop"
                    $InstalledMpv = $true
                }
            } catch {}
        }
        if (-not $InstalledMpv -and (Get-Command "choco" -ErrorAction SilentlyContinue)) {
            Write-Step "Installing mpv via chocolatey..."
            try {
                $p = Start-Process -FilePath "choco" -ArgumentList "install mpv -y" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed mpv player via chocolatey"
                    $InstalledMpv = $true
                }
            } catch {}
        }
        if (-not $InstalledMpv) {
            Write-Warn "Could not auto-install mpv (recommended for streaming). You can install it with: winget install io.mpv"
        }
    }

    # 2. yt-dlp streaming engine
    $HasYtDlp = (Get-Command "yt-dlp" -ErrorAction SilentlyContinue) -or `
                (Test-Path (Join-Path $TargetBinDir "yt-dlp.exe"))

    if ($HasYtDlp) {
        Write-Success "yt-dlp streaming engine is already installed"
    } else {
        $InstalledYtDlp = $false
        if (Get-Command "winget" -ErrorAction SilentlyContinue) {
            Write-Step "Installing yt-dlp via winget..."
            try {
                $p = Start-Process -FilePath "winget" -ArgumentList "install --id yt-dlp.yt-dlp --source winget --accept-source-agreements --accept-package-agreements --silent" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed yt-dlp via winget"
                    $InstalledYtDlp = $true
                }
            } catch {}
        }

        if (-not $InstalledYtDlp) {
            # Direct binary fallback into user install directory
            try {
                if (-not (Test-Path $TargetBinDir)) {
                    New-Item -ItemType Directory -Force -Path $TargetBinDir | Out-Null
                }
                $DestYtDlp = Join-Path $TargetBinDir "yt-dlp.exe"
                Write-Step "Downloading standalone yt-dlp.exe to $DestYtDlp..."
                Invoke-WebRequest -Uri "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" -OutFile $DestYtDlp -UseBasicParsing
                try { Unblock-File -Path $DestYtDlp -ErrorAction SilentlyContinue } catch {}
                Write-Success "Installed yt-dlp directly to $DestYtDlp"
                $InstalledYtDlp = $true
            } catch {
                Write-Warn "Could not auto-download yt-dlp.exe: $_"
            }
        }
    }

    # 3. ffmpeg audio/video muxer
    $HasFfmpeg = (Get-Command "ffmpeg" -ErrorAction SilentlyContinue) -or `
                 (Test-Path (Join-Path $TargetBinDir "ffmpeg.exe"))

    if ($HasFfmpeg) {
        Write-Success "ffmpeg audio/video multiplexer is already installed"
    } else {
        $InstalledFfmpeg = $false
        if (Get-Command "winget" -ErrorAction SilentlyContinue) {
            Write-Step "Installing ffmpeg via winget..."
            try {
                $p = Start-Process -FilePath "winget" -ArgumentList "install --id Gyan.FFmpeg --source winget --accept-source-agreements --accept-package-agreements --silent" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed ffmpeg via winget"
                    $InstalledFfmpeg = $true
                }
            } catch {}
        }
        if (-not $InstalledFfmpeg -and (Get-Command "scoop" -ErrorAction SilentlyContinue)) {
            try {
                $p = Start-Process -FilePath "scoop" -ArgumentList "install ffmpeg" -NoNewWindow -Wait -PassThru
                if ($p.ExitCode -eq 0) {
                    Write-Success "Installed ffmpeg via scoop"
                    $InstalledFfmpeg = $true
                }
            } catch {}
        }
        if (-not $InstalledFfmpeg) {
            Write-Warn "ffmpeg not found (recommended for multi-res stream merging). You can install it with: winget install Gyan.FFmpeg"
        }
    }
}

Install-Prerequisites -TargetBinDir $EffectiveInstallDir

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("sumanmovies-tui-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

$ZipFile = Join-Path $TempDir $ArchiveName
$ChecksumFile = Join-Path $TempDir "SHA256SUMS"
$BaseUrl = "https://github.com/$Repo/releases/download/$TargetVersion"
$Url = "$BaseUrl/$ArchiveName"

try {
    Write-Step "[3/5] Downloading $ArchiveName..."
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile -UseBasicParsing
    Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -OutFile $ChecksumFile -UseBasicParsing
    try { Unblock-File -Path $ZipFile -ErrorAction SilentlyContinue } catch {}
    Write-Success "[3/5] Downloaded $ArchiveName"

    Write-Step "[4/5] Verifying SHA256 checksum..."
    $ChecksumLine = Get-Content $ChecksumFile | Where-Object { $_ -match "\s+$([regex]::Escape($ArchiveName))$" } | Select-Object -First 1
    if (-not $ChecksumLine) {
        throw "Release checksum is missing for $ArchiveName."
    }
    $ExpectedHash = ($ChecksumLine -split "\s+")[0].Trim().ToUpper()
    $ActualHash = (Get-FileHash -Path $ZipFile -Algorithm SHA256).Hash.Trim().ToUpper()
    if ($ActualHash -ne $ExpectedHash) {
        throw "Checksum verification failed."
    }
    Write-Success "[4/5] Cryptographic checksum verified"

    Write-Step "[5/5] Installing binary to $EffectiveInstallDir..."
    if (-not (Test-Path $EffectiveInstallDir)) {
        New-Item -ItemType Directory -Force -Path $EffectiveInstallDir | Out-Null
    }

    $RunningProcesses = Get-Process -Name "sumanmovies-tui","sumanmovies" -ErrorAction SilentlyContinue
    if ($RunningProcesses) {
        $RunningProcesses | Stop-Process -Force
        Start-Sleep -Seconds 1
    }

    Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force
    $ExtractedExe = Join-Path $TempDir $BinName
    if (-not (Test-Path $ExtractedExe)) {
        $AltExe = Join-Path $TempDir "sumanmovies-tui.exe"
        if (Test-Path $AltExe) {
            $ExtractedExe = $AltExe
        } else {
            throw "Binary not found in archive."
        }
    }

    try { Unblock-File -Path $ExtractedExe -ErrorAction SilentlyContinue } catch {}
    Copy-Item -Path $ExtractedExe -Destination $ExePath -Force
    try { Unblock-File -Path $ExePath -ErrorAction SilentlyContinue } catch {}

    # Provide alias binary so both sumanmovies and sumanmovies-tui execute smoothly
    $TuiAliasPath = Join-Path $EffectiveInstallDir "sumanmovies-tui.exe"
    if ($ExePath -ne $TuiAliasPath) {
        Copy-Item -Path $ExePath -Destination $TuiAliasPath -Force -ErrorAction SilentlyContinue
    }

    try {
        $SmokeOutput = (& $ExePath --version 2>&1 | Out-String)
        if ($LASTEXITCODE -ne 0) {
            throw "Binary execution test failed: $SmokeOutput"
        }
    } catch {
        throw "Binary execution test failed: $_"
    }
    Write-Success "[5/5] Binary installed to $ExePath"
} catch {
    Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Err "Installation failed: $_"
} finally {
    Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

$PathModified = $false
if (-not $NoModifyPath) {
    if (Add-ToUserPath -Directory $EffectiveInstallDir) {
        $PathModified = $true
    }
    if (@($env:PATH -split ";") -notcontains $EffectiveInstallDir.TrimEnd("\")) {
        $env:PATH = "$env:PATH;$EffectiveInstallDir"
    }
}

$PlayerDetected = ""
if ((Get-Command "mpv" -ErrorAction SilentlyContinue) -or (Test-Path "C:\Program Files\mpv\mpv.exe") -or (Test-Path "C:\Program Files\MPV Player\mpv.exe") -or (Test-Path "C:\mpv\mpv.exe") -or (Test-Path "$env:LOCALAPPDATA\Programs\mpv\mpv.exe")) {
    $PlayerDetected = "mpv"
} elseif ((Get-Command "vlc" -ErrorAction SilentlyContinue) -or (Test-Path "C:\Program Files\VideoLAN\VLC\vlc.exe") -or (Test-Path "C:\Program Files (x86)\VideoLAN\VLC\vlc.exe") -or (Test-Path "$env:LOCALAPPDATA\Programs\VLC\vlc.exe")) {
    $PlayerDetected = "VLC"
}

$YtDlpDetected = (Get-Command "yt-dlp" -ErrorAction SilentlyContinue) -or (Test-Path (Join-Path $EffectiveInstallDir "yt-dlp.exe"))
$FfmpegDetected = (Get-Command "ffmpeg" -ErrorAction SilentlyContinue) -or (Test-Path (Join-Path $EffectiveInstallDir "ffmpeg.exe"))

Write-Host ""
Write-Host "  + SumanMovies-Tui $TargetVersion successfully installed!" -ForegroundColor Green
Write-Host ""
Write-Host "  - Binary:  " -ForegroundColor DarkGray -NoNewline
Write-Host $ExePath -ForegroundColor White

if ($PlayerDetected) {
    Write-Host "  - Player:  " -ForegroundColor DarkGray -NoNewline
    Write-Host "$PlayerDetected (ready)" -ForegroundColor Green
} else {
    Write-Host "  - Player:  " -ForegroundColor DarkGray -NoNewline
    Write-Host "None detected (mpv recommended)" -ForegroundColor Cyan
}

if ($YtDlpDetected) {
    Write-Host "  - Engine:  " -ForegroundColor DarkGray -NoNewline
    Write-Host "yt-dlp (ready)" -ForegroundColor Green
} else {
    Write-Host "  - Engine:  " -ForegroundColor DarkGray -NoNewline
    Write-Host "yt-dlp (optional)" -ForegroundColor Cyan
}

if ($FfmpegDetected) {
    Write-Host "  - Codec:   " -ForegroundColor DarkGray -NoNewline
    Write-Host "ffmpeg (ready)" -ForegroundColor Green
} else {
    Write-Host "  - Codec:   " -ForegroundColor DarkGray -NoNewline
    Write-Host "ffmpeg (optional)" -ForegroundColor Cyan
}

if ($PathModified) {
    Write-Host "  - Shell:   " -ForegroundColor DarkGray -NoNewline
    Write-Host "PATH updated in User Environment" -ForegroundColor Magenta
}

Write-Host ""
Write-Host "  To start streaming:" -ForegroundColor White
Write-Host "    $ sumanmovies" -ForegroundColor Green
Write-Host ""

if (-not $PlayerDetected) {
    Write-Host "  [i] Note: mpv media player is recommended for video playback.`n" -ForegroundColor Cyan
}

if ($PathModified) {
    Write-Host "  [i] Restart your terminal window for the updated PATH to take effect in other sessions.`n" -ForegroundColor Cyan
}
