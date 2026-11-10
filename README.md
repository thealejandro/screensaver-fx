# Screensaver FX

Protector de pantalla nativo para Windows (`.scr`) y macOS (`.saver`) con las animaciones del screensaver de Omarchy y 20 efectos nuevos. El núcleo está en Rust (`core/`, sin dependencias de plataforma) y cada sistema tiene un envoltorio fino. No usa ningún motor web y ocupa unos 5 MB de RAM. En Windows deja de dibujar mientras la pantalla está apagada.

## Instalar

### Windows 10/11

- **Instalador:** descarga `ScreensaverFX-Setup.exe` de [Releases](https://github.com/thealejandro/screensaver-fx/releases/latest). Permite personalizar el rótulo, los colores y los efectos con una vista previa en vivo.
- **Terminal** (sin permisos de administrador):

  ```powershell
  irm https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.ps1 | iex
  ```

  Para desinstalar: `& ([scriptblock]::Create((irm https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.ps1))) -Uninstall`

«Para todos los usuarios» (pide administrador) lo deja en `System32`, en la lista de protectores de Windows para cualquier cuenta; «Solo para mí» no pide permisos.

Algunas apps en primer plano bloquean el protector de Windows aunque no estén reproduciendo nada. Por eso se instala un vigilante mínimo (`ScreensaverFX.exe /watch`, ~1,3 MB, arranca con la sesión): si pasa el tiempo de espera y Windows no lanzó el protector configurado, lo lanza él. Respeta lo mismo que Windows: vídeo o videollamadas, presentaciones, pantalla completa, sesión bloqueada y «Mostrar la pantalla de inicio de sesión al reanudar».

Lo que hace queda en `%APPDATA%\ScreensaverFX\watch.log` (por qué lo lanzó o por qué no).

### Actualizaciones automáticas

Screensaver FX se actualiza solo desde las [releases](https://github.com/thealejandro/screensaver-fx/releases), sin tocar tu configuración ni el protector que tengas elegido:

- **Para todos los usuarios**: la tarea programada `ScreensaverFX\Update` (como SYSTEM) busca al arrancar el equipo y una vez al día. Registro en `%ProgramData%\ScreensaverFX\update.log`.
- **Solo para mí**: el vigilante busca a los 3 minutos de iniciar sesión y luego cada 24 h. Registro en `watch.log`.

Nunca instala con el protector o la configuración abiertos. El instalador se descarga por HTTPS y se comprueba con la suma SHA-256 publicada en la release antes de ejecutarlo. Para buscar a mano: `ScreensaverFX.exe /update`.

El instalador no está firmado, así que SmartScreen puede avisar: «Más información» › «Ejecutar de todas formas».

### macOS 11 o posterior

- **Arrastrar:** abre `ScreensaverFX.dmg` de [Releases](https://github.com/thealejandro/screensaver-fx/releases/latest) y arrastra el protector sobre «Screen Savers» (Finder pide tu contraseña porque se instala para todos los usuarios).
- **Instalador:** `ScreensaverFX.pkg`, de la misma página.
- **Terminal:**

  ```bash
  curl -fsSL https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.sh | bash
  ```

Ni el .pkg ni el .dmg están notarizados. Si macOS bloquea el instalador o el protector, ve a Ajustes del Sistema › Privacidad y seguridad › «Abrir igualmente». Con el método de la terminal no aparece el aviso.

## Varios monitores

Todos los monitores van sincronizados: el mismo efecto y el mismo tema a la vez en cada pantalla. Tres efectos usan el escritorio entero y pasan de un monitor al siguiente, de izquierda a derecha, con cualquier número de pantallas y aunque tengan tamaños, escalas u orientaciones distintas:

- **Relevo:** las letras del rótulo saltan en arco de una pantalla a la siguiente y al final se encienden en todas.
- **Cometa:** un cometa cruza el escritorio, ilumina el rótulo de cada pantalla al pasar y deja una estela de chispas.
- **Marea:** una ola de color recorre todos los monitores, va encendiendo los rótulos y vuelve.

Con un solo monitor también funcionan, dentro de esa pantalla.

## Compilar

### macOS

Requisitos: [rustup](https://rustup.rs) y las Command Line Tools (`xcode-select --install`).

```bash
./macos/build.sh --install     # compila e instala en ~/Library/Screen Savers
./macos/package.sh             # dist/ScreensaverFX-macOS.zip, ScreensaverFX-<versión>.pkg y .dmg
```

### Windows

Requisitos: rustup con el toolchain `stable-x86_64-pc-windows-gnullvm`, [llvm-mingw](https://github.com/mstorsjo/llvm-mingw) en el PATH e [Inno Setup 6.7+](https://jrsoftware.org/isinfo.php) para el instalador.

```bash
cargo build -p screensaver-fx --release
cargo build -p screensaver-fx --release --target aarch64-pc-windows-gnullvm
ISCC installer\windows\ScreensaverFX.iss          # dist\ScreensaverFX-Setup-<versión>.exe
```

Argumentos del `.scr`: `/s` pantalla completa, `/p <hwnd>` vista previa, `/c` configuración, `/w` ventana de prueba. La configuración vive en `%APPDATA%\ScreensaverFX\config.ini`.

### Publicar una versión

```bash
git tag v0.1.0 && git push --tags
```

GitHub Actions compila Windows (x64 + ARM64) y macOS (universal) y sube los instaladores a Releases, con una tabla de qué descargar según el equipo y los cambios `feat`/`fix`/`perf` desde la versión anterior (`.github/release-notes.sh`).

## Herramientas

```bash
cargo run -p ssfx-core --example snap --release -- snaps auto 1920 1080 "banner=text|title=Hola"   # fotogramas a PNG
cargo run -p ssfx-core --example brand --release -- installer/assets                              # icono e imágenes
cargo run -p ssfx-core --example desk --release -- desk relay,comet,tide 8                          # escritorio de 3 monitores a PNG
```
