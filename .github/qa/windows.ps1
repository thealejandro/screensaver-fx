# QA del instalador de Windows en una máquina limpia (CI): instala, prueba, actualiza y desinstala.
#   pwsh .github/qa/windows.ps1 -Setup dist/ScreensaverFX-Setup.exe
param([Parameter(Mandatory)] [string]$Setup)
$ErrorActionPreference = 'Stop'
$Setup = (Resolve-Path $Setup).Path
$app = "$env:LOCALAPPDATA\Programs\ScreensaverFX"
$exe = "$app\ScreensaverFX.exe"
$data = "$env:APPDATA\ScreensaverFX"
$desk = 'HKCU:\Control Panel\Desktop'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$script:fails = 0

function Check([string]$name, [bool]$ok, [string]$detail = '') {
  if ($ok) { Write-Host "  OK    $name" } else { Write-Host "  FALLA $name $detail" -ForegroundColor Red; $script:fails++ }
}
function Warn([string]$name, [bool]$ok) { if ($ok) { Write-Host "  OK    $name" } else { Write-Host "  AVISO $name" -ForegroundColor Yellow } }
function Procs([string]$pattern) { @(Get-CimInstance Win32_Process -Filter "Name like 'ScreensaverFX%'" | Where-Object { $_.CommandLine -match $pattern }) }
function WaitFor([scriptblock]$cond, [int]$sec = 10) { for ($i = 0; $i -lt $sec * 4; $i++) { if (& $cond) { return $true }; Start-Sleep -Milliseconds 250 }; return $false }
function RunSetup([string]$extra) {
  $p = Start-Process $Setup -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER $extra" -PassThru
  $p.WaitForExit(180000) | Out-Null
  return $p.ExitCode
}
# Arquitectura de un PE: 0x8664 x64, 0xAA64 ARM64.
function Machine([string]$path) {
  $b = [IO.File]::ReadAllBytes($path); $pe = [BitConverter]::ToInt32($b, 0x3C); return [BitConverter]::ToUInt16($b, $pe + 4)
}
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Native {
  [DllImport("user32.dll")] public static extern void keybd_event(byte v, byte s, uint f, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
"@

$arch = $env:PROCESSOR_ARCHITECTURE
$want = if ($arch -eq 'ARM64') { 0xAA64 } else { 0x8664 }
Write-Host "Máquina: $arch · Windows $([Environment]::OSVersion.Version)"

Write-Host "1) Instalación limpia (solo para mí, activando el protector)"
Check "el instalador termina bien" ((RunSetup '') -eq 0)
Check "ScreensaverFX.exe y .scr instalados" ((Test-Path $exe) -and (Test-Path "$app\ScreensaverFX.scr"))
Check "binario nativo de la máquina ($arch)" ((Machine $exe) -eq $want) ("0x{0:X}" -f (Machine $exe))
Check "desinstalador registrado" (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{6B0E3F2A-5C1D-4E8B-9A7F-3D2C1B0A9E8F}_is1')
Check "accesos directos en el menú Inicio" (@(Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Filter '*Screensaver FX*.lnk').Count -eq 2)
Check "config.ini creada" (Test-Path "$data\config.ini")
$d = Get-ItemProperty $desk
Check "protector seleccionado en Windows" ($d.'SCRNSAVE.EXE' -eq "$app\ScreensaverFX.scr") $d.'SCRNSAVE.EXE'
Check "protector activo" ($d.ScreenSaveActive -eq '1')
Check "arranque con la sesión (Run)" ((Get-ItemProperty $runKey).ScreensaverFX -eq "`"$exe`" /watch")

Write-Host "2) Vigilante"
Check "el instalador lo dejó en marcha" (WaitFor { (Procs '/watch').Count -eq 1 })
$w = Procs '/watch' | Select-Object -First 1
if ($w) {
  $mb = [math]::Round((Get-Process -Id $w.ProcessId).PrivateMemorySize64 / 1MB, 2)
  Check "RAM privada < 4 MB ($mb MB)" ($mb -lt 4)
}
Check "watch.log escrito" (WaitFor { (Test-Path "$data\watch.log") -and (Select-String -Path "$data\watch.log" -Pattern 'vigilante iniciado' -Quiet) })
Start-Process $exe -ArgumentList '/watch'
Start-Sleep 2
Check "una sola instancia por sesión" ((Procs '/watch').Count -eq 1)
[void](Start-Process $exe -ArgumentList '/unwatch' -PassThru).WaitForExit(15000)
Check "/unwatch lo detiene" (WaitFor { (Procs '/watch').Count -eq 0 } 5)
# Sin etiqueta (0.0.0) no busca; con etiqueta consulta la release publicada (que es más antigua).
$u = Start-Process $exe -ArgumentList '/update' -PassThru
Check "/update termina" ($u.WaitForExit(90000))
Check "/update deja constancia" (Select-String -Path "$data\watch.log" -Pattern 'actualizaciones: (al día|compilación de desarrollo)' -Quiet) ((Get-Content "$data\watch.log" -Tail 2) -join ' | ')

Write-Host "3) Protector, vista previa y configuración"
$s = Start-Process $exe -ArgumentList '/s' -PassThru
Start-Sleep 4
Check "/s sigue abierto tras 4 s (sin cierres inesperados)" (-not $s.HasExited) "código $($s.ExitCode)"
if (-not $s.HasExited) {
  [Native]::keybd_event(0x10, 0, 0, [UIntPtr]::Zero); [Native]::keybd_event(0x10, 0, 2, [UIntPtr]::Zero)
  $closed = WaitFor { $s.HasExited } 3
  Warn "se cierra con una tecla sin tener el foco (necesita escritorio interactivo)" $closed
  if (-not $closed) { Stop-Process -Id $s.Id -Force }
}
$c = Start-Process $exe -ArgumentList '/c' -PassThru
Check "/c abre la ventana de configuración" (WaitFor { $c.Refresh(); $c.MainWindowHandle -ne [IntPtr]::Zero } 8)
if (-not $c.HasExited) {
  [void][Native]::PostMessage($c.MainWindowHandle, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) # WM_CLOSE
  Check "la configuración se cierra limpia" (WaitFor { $c.HasExited } 5)
  if (-not $c.HasExited) { Stop-Process -Id $c.Id -Force }
}
$w2 = Start-Process $exe -ArgumentList '/w' -PassThru
Start-Sleep 3
Check "/w (ventana) dibuja sin cerrarse" (-not $w2.HasExited)
if (-not $w2.HasExited) { Stop-Process -Id $w2.Id -Force }

Write-Host "4) Actualización sobre la misma versión, conservando la configuración"
Add-Content "$data\config.ini" "title=QA prueba"
$cfg = Get-Content "$data\config.ini" -Raw
Start-Process $exe -ArgumentList '/watch'
Check "vigilante en marcha antes de actualizar" (WaitFor { (Procs '/watch').Count -eq 1 })
Check "la actualización termina bien" ((RunSetup '/NOACTIVATE') -eq 0)
Check "configuración conservada" ((Get-Content "$data\config.ini" -Raw) -match 'QA prueba')
Check "vigilante reiniciado (uno solo)" (WaitFor { (Procs '/watch').Count -eq 1 })
Check "protector sigue seleccionado" ((Get-ItemProperty $desk).'SCRNSAVE.EXE' -eq "$app\ScreensaverFX.scr")

Write-Host "5) Desinstalación"
$u = Start-Process "$app\unins000.exe" -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -PassThru
$u.WaitForExit(60000) | Out-Null
Check "carpeta eliminada" (WaitFor { -not (Test-Path $app) } 30)
Check "vigilante detenido" ((Procs '/watch').Count -eq 0)
Check "Run eliminado" ($null -eq (Get-ItemProperty $runKey).ScreensaverFX)
Check "protector deseleccionado" ([string]::IsNullOrEmpty((Get-ItemProperty $desk).'SCRNSAVE.EXE'))
Check "accesos directos eliminados" (@(Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Filter '*Screensaver FX*.lnk').Count -eq 0)
Check "configuración del usuario conservada (silencioso = no borrar)" (Test-Path "$data\config.ini")

Write-Host "6) Para todos: tarea de actualización como SYSTEM"
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Warn "requiere una sesión de administrador" $false } else {
  $pf = "$env:ProgramFiles\ScreensaverFX"
  $ulog = "$env:ProgramData\ScreensaverFX\update.log"
  $p = Start-Process $Setup -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /ALLUSERS /NOACTIVATE' -PassThru
  $p.WaitForExit(180000) | Out-Null
  Check "instalación para todos" (($p.ExitCode -eq 0) -and (Test-Path "$pf\ScreensaverFX.exe"))
  schtasks /Query /TN 'ScreensaverFX\Update' *> $null
  Check "tarea programada creada" ($LASTEXITCODE -eq 0)
  schtasks /Run /TN 'ScreensaverFX\Update' *> $null
  Check "la tarea corre como SYSTEM y deja constancia" (WaitFor { (Test-Path $ulog) -and (Select-String -Path $ulog -Pattern 'actualizaciones:' -Quiet) } 60)
  $u = Start-Process "$pf\unins000.exe" -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -PassThru
  $u.WaitForExit(60000) | Out-Null
  Check "desinstalada" (WaitFor { -not (Test-Path "$pf\ScreensaverFX.exe") } 30)
  schtasks /Query /TN 'ScreensaverFX\Update' *> $null
  Check "tarea eliminada" ($LASTEXITCODE -ne 0)
}

Write-Host "Resultado: $script:fails fallos"
exit $script:fails
