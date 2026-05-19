$ErrorActionPreference = "SilentlyContinue"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path
$stateDir = Join-Path $scriptDir ".state"
$vitePidFile = Join-Path $stateDir "vite.pid"
$tauriPidFile = Join-Path $stateDir "tauri.pid"
$debugAppPath = (Join-Path $projectRoot "src-tauri\target\debug\app.exe").ToLowerInvariant()

function Stop-ProcessTree {
  param([int]$ProcessId)

  if ($ProcessId -le 0) {
    return
  }

  cmd /c "taskkill /PID $ProcessId /T /F" | Out-Null
}

function Stop-FromPidFile {
  param([string]$PidFile)

  if (-not (Test-Path $PidFile)) {
    return
  }

  $raw = (Get-Content $PidFile -Raw).Trim()
  if ($raw) {
    Stop-ProcessTree -ProcessId ([int]$raw)
  }

  Remove-Item $PidFile -ErrorAction SilentlyContinue
}

function Get-DevShellProcesses {
  Get-CimInstance Win32_Process | Where-Object {
    $commandLine = $_.CommandLine
    $exePath = $_.ExecutablePath

    ($_.Name -eq "app.exe" -and $exePath -and $exePath.ToLowerInvariant() -eq $debugAppPath) -or
    ($_.Name -eq "cargo.exe" -and $commandLine -and
      $commandLine -like "*run --no-default-features --color always --*") -or
    ($_.Name -eq "cmd.exe" -and $commandLine -and (
      $commandLine -like "*run-tauri-dev-shell.cmd*" -or
      $commandLine -like "*taskkill /PID*" -or
      $commandLine -like "*npm.cmd run dev*" -or
      $commandLine -like "*vite*" -or
      $commandLine -like "*tauri dev --config src-tauri/tauri.dev-shell.conf.json*"
    )) -or
    ($_.Name -eq "node.exe" -and $commandLine -and (
      $commandLine -like "*npm-cli.js*run tauri:dev:shell*" -or
      $commandLine -like "*@tauri-apps\\cli\\tauri.js*dev --config src-tauri/tauri.dev-shell.conf.json*" -or
      ($commandLine -like "*npm-cli.js*run dev*" -and $commandLine -like "*$projectRoot*") -or
      ($commandLine -like "*vite\\bin\\vite.js*" -and $commandLine -like "*$projectRoot*")
    ))
  }
}

Stop-FromPidFile -PidFile $tauriPidFile
Stop-FromPidFile -PidFile $vitePidFile

foreach ($process in (Get-DevShellProcesses | Sort-Object ProcessId -Descending | Select-Object -Unique ProcessId,Name)) {
  try {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
  } catch {
  }
}

try {
  $portOwner = Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction Stop |
    Select-Object -First 1 -ExpandProperty OwningProcess
  if ($portOwner) {
    Stop-ProcessTree -ProcessId $portOwner
  }
} catch {
}

Get-ChildItem $stateDir -Force -ErrorAction SilentlyContinue |
  Remove-Item -Force -ErrorAction SilentlyContinue

Write-Host "Dev shell stopped."
