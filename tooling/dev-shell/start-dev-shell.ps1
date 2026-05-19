$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path
$stateDir = Join-Path $scriptDir ".state"
$logDir = Join-Path $scriptDir "logs"
$vitePidFile = Join-Path $stateDir "vite.pid"
$tauriPidFile = Join-Path $stateDir "tauri.pid"
$tauriCmdFile = Join-Path $stateDir "run-tauri-dev-shell.cmd"
$debugAppPath = (Join-Path $projectRoot "src-tauri\target\debug\app.exe").ToLowerInvariant()

New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

function Test-RunningProcess {
  param([string]$PidFile)

  if (-not (Test-Path $PidFile)) {
    return $null
  }

  $raw = (Get-Content $PidFile -Raw).Trim()
  if (-not $raw) {
    Remove-Item $PidFile -ErrorAction SilentlyContinue
    return $null
  }

  try {
    $process = Get-Process -Id ([int]$raw) -ErrorAction Stop
    return $process
  } catch {
    Remove-Item $PidFile -ErrorAction SilentlyContinue
    return $null
  }
}

function Get-PortOwner {
  param([int]$Port)

  try {
    return Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction Stop |
      Select-Object -First 1 -ExpandProperty OwningProcess
  } catch {
    return $null
  }
}

function Get-ExistingDevShellProcesses {
  Get-CimInstance Win32_Process | Where-Object {
    $commandLine = $_.CommandLine
    $exePath = $_.ExecutablePath

    ($_.Name -eq "app.exe" -and $exePath -and $exePath.ToLowerInvariant() -eq $debugAppPath) -or
    ($_.Name -eq "cargo.exe" -and $commandLine -and
      $commandLine -like "*run --no-default-features --color always --*") -or
    ($_.Name -eq "cmd.exe" -and $commandLine -and (
      $commandLine -like "*run-tauri-dev-shell.cmd*" -or
      $commandLine -like "*tauri dev --config src-tauri/tauri.dev-shell.conf.json*"
    )) -or
    ($_.Name -eq "node.exe" -and $commandLine -and (
      $commandLine -like "*npm-cli.js*run tauri:dev:shell*" -or
      $commandLine -like "*@tauri-apps\\cli\\tauri.js*dev --config src-tauri/tauri.dev-shell.conf.json*"
    ))
  }
}

function Wait-PortReady {
  param(
    [int]$Port,
    [int]$TimeoutSeconds = 20
  )

  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    if (Get-PortOwner -Port $Port) {
      return $true
    }
    Start-Sleep -Milliseconds 250
  }

  return $false
}

$existingTauri = Test-RunningProcess -PidFile $tauriPidFile
if ($existingTauri) {
  Write-Host "Dev shell is already running. PID: $($existingTauri.Id)"
  exit 0
}

$existingShellProcesses = @(Get-ExistingDevShellProcesses)
if ($existingShellProcesses.Count -gt 0) {
  $processSummary = ($existingShellProcesses | Select-Object -ExpandProperty ProcessId) -join ", "
  Write-Host "Dev shell appears to be running already. Processes: $processSummary"
  exit 0
}

$vitePortOwner = Get-PortOwner -Port 1420
if (-not $vitePortOwner) {
  $viteOut = Join-Path $logDir "vite.out.log"
  $viteErr = Join-Path $logDir "vite.err.log"
  $viteProcess = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/c", "npm.cmd run dev" `
    -WorkingDirectory $projectRoot `
    -WindowStyle Hidden `
    -RedirectStandardOutput $viteOut `
    -RedirectStandardError $viteErr `
    -PassThru
  Set-Content -Path $vitePidFile -Value $viteProcess.Id

  if (-not (Wait-PortReady -Port 1420)) {
    throw "Vite dev server did not become ready within 20 seconds. Check $viteErr"
  }
} else {
  Set-Content -Path $vitePidFile -Value $vitePortOwner
}

$vsDevCmd = Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio" `
  -Recurse `
  -Filter "VsDevCmd.bat" `
  -ErrorAction SilentlyContinue |
  Select-Object -First 1 -ExpandProperty FullName

if (-not $vsDevCmd) {
  throw "VsDevCmd.bat was not found. Please install Visual Studio Build Tools."
}

@"
@echo off
call "$vsDevCmd" -arch=x64 -host_arch=x64 >nul
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
set CARGO_HTTP_CHECK_REVOKE=false
cd /d "$projectRoot"
npm.cmd run tauri:dev:shell
"@ | Set-Content -Path $tauriCmdFile -Encoding ASCII

$tauriOut = Join-Path $logDir "tauri.out.log"
$tauriErr = Join-Path $logDir "tauri.err.log"
$tauriProcess = Start-Process -FilePath "cmd.exe" `
  -ArgumentList "/c", "`"$tauriCmdFile`"" `
  -WorkingDirectory $projectRoot `
  -WindowStyle Hidden `
  -RedirectStandardOutput $tauriOut `
  -RedirectStandardError $tauriErr `
  -PassThru

Set-Content -Path $tauriPidFile -Value $tauriProcess.Id

Write-Host "Dev shell started."
Write-Host "Vite log: $viteOut"
Write-Host "Tauri log: $tauriOut"
