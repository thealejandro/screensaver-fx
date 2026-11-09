# Screensaver FX: instalación desde la terminal (Windows 10/11, sin permisos de administrador).
#   irm https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.ps1 | iex
# Opciones (descargando el script):  .\install.ps1 [-NoActivate] [-Uninstall]
param([switch]$NoActivate, [switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$repo = 'thealejandro/screensaver-fx'
$appId = '{6B0E3F2A-5C1D-4E8B-9A7F-3D2C1B0A9E8F}_is1'

function Say($msg) { Write-Host "  $msg" -ForegroundColor Cyan }

Write-Host ''
Write-Host '  ❯ Screensaver FX' -ForegroundColor Magenta
Write-Host ''

if ($Uninstall) {
  $keys = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$appId",
          "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$appId"
  $key = $keys | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $key) { Say 'No está instalado.'; return }
  $cmd = (Get-ItemProperty $key).UninstallString.Trim('"')
  Say 'Desinstalando…'
  Start-Process $cmd -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait
  Say 'Listo. Tu configuración sigue en %APPDATA%\ScreensaverFX.'
  return
}

$setup = Join-Path $env:TEMP 'ScreensaverFX-Setup.exe'
Say 'Descargando la última versión…'
Invoke-WebRequest "https://github.com/$repo/releases/latest/download/ScreensaverFX-Setup.exe" -OutFile $setup -UseBasicParsing

$setupArgs = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER')
if ($NoActivate) { $setupArgs += '/NOACTIVATE' }
Say 'Instalando…'
$p = Start-Process $setup -ArgumentList $setupArgs -Wait -PassThru
Remove-Item $setup -ErrorAction SilentlyContinue
if ($p.ExitCode -ne 0) { throw "El instalador terminó con código $($p.ExitCode)." }

Say 'Instalado. Configúralo desde Inicio › «Configurar Screensaver FX».'
if (-not $NoActivate) { Say 'Ya es tu protector de pantalla.' }
Write-Host ''
