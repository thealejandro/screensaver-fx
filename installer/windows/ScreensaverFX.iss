; Instalador de Screensaver FX (Inno Setup 6.7+).
;   ISCC installer\windows\ScreensaverFX.iss
;   ISCC /DAppVersion=0.2.0 /DX64Bin=... /DArm64Bin=... installer\windows\ScreensaverFX.iss
; Las imágenes y el icono los genera: cargo run -p ssfx-core --example brand --release

#ifndef AppVersion
  #define AppVersion "0.1.14"
#endif
#ifndef X64Bin
  #define X64Bin "..\..\target\release\ScreensaverFX.exe"
#endif
#ifndef Arm64Bin
  #define Arm64Bin "..\..\target\aarch64-pc-windows-gnullvm\release\ScreensaverFX.exe"
#endif
#define Assets "..\assets"
#define HaveArm64 FileExists(AddBackslash(SourcePath) + Arm64Bin) || FileExists(Arm64Bin)

[Setup]
AppId={{6B0E3F2A-5C1D-4E8B-9A7F-3D2C1B0A9E8F}
AppName=Screensaver FX
AppVersion={#AppVersion}
AppVerName=Screensaver FX {#AppVersion}
AppPublisher=thealejandro.dev
AppPublisherURL=https://thealejandro.dev
AppSupportURL=https://github.com/thealejandro/screensaver-fx/issues
AppUpdatesURL=https://github.com/thealejandro/screensaver-fx/releases
AppCopyright=MIT · thealejandro.dev
VersionInfoVersion={#AppVersion}
VersionInfoDescription=Instalador de Screensaver FX
; Sin espacios: Windows guarda la ruta del protector sin comillas.
DefaultDirName={autopf}\ScreensaverFX
DisableWelcomePage=no
DisableDirPage=yes
DisableProgramGroupPage=yes
DefaultGroupName=Screensaver FX
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern dark
WizardSizePercent=110
WizardImageFile={#Assets}\wizard-164.png,{#Assets}\wizard-246.png,{#Assets}\wizard-328.png
WizardSmallImageFile={#Assets}\small-55.png,{#Assets}\small-83.png,{#Assets}\small-110.png
SetupIconFile={#Assets}\icon.ico
UninstallDisplayIcon={app}\ScreensaverFX.exe
UninstallDisplayName=Screensaver FX
OutputDir=..\..\dist
OutputBaseFilename=ScreensaverFX-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
ShowLanguageDialog=auto
CloseApplications=no

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Messages]
es.WelcomeLabel2=Esto instalará [name/ver]: el protector de pantalla de Omarchy con 35 animaciones, nativo y ligero (≈5 MB de RAM).%n%nEn el siguiente paso podrás personalizar el rótulo, los colores y los efectos con una vista previa en vivo.
en.WelcomeLabel2=This will install [name/ver]: the Omarchy screensaver with 35 animations, native and lightweight (≈5 MB of RAM).%n%nIn the next step you can customize the banner, colors and effects with a live preview.

[CustomMessages]
es.PageTitle=Personaliza tu protector
en.PageTitle=Make it yours
es.PageDesc=Los cambios se ven al instante en la vista previa. Podrás cambiarlos luego desde Configuración.
en.PageDesc=Changes show up instantly in the preview. You can change them later from Settings.
es.LBanner=Rótulo
en.LBanner=Banner
es.BannerDefault=JACC · XACODE (por defecto)
en.BannerDefault=JACC · XACODE (default)
es.BannerText=Texto propio
en.BannerText=Custom text
es.BannerFile=Arte ASCII (screensaver.txt)
en.BannerFile=ASCII art (screensaver.txt)
es.LTitle=Título (grande)
en.LTitle=Title (large)
es.LSubtitle=Subtítulo (opcional)
en.LSubtitle=Subtitle (optional)
es.LTheme=Colores
en.LTheme=Colors
es.ThemeAuto=Color de acento de Windows
en.ThemeAuto=Windows accent color
es.ThemeRandom=Aleatorio en cada inicio
en.ThemeRandom=Random on every start
es.LEffects=Efectos
en.LEffects=Effects
es.FxAll=Todos (35)
en.FxAll=All (35)
es.FxClassic=Clásicos de Omarchy (6)
en.FxClassic=Omarchy classics (6)
es.FxModern=Nuevos (10)
en.FxModern=New (10)
es.FxCustom=Selección actual
en.FxCustom=Current selection
es.LSize=Tamaño del rótulo
en.LSize=Banner size
es.SizeS=Pequeño
en.SizeS=Small
es.SizeM=Mediano
en.SizeM=Medium
es.SizeL=Grande
en.SizeL=Large
es.LPreview=Vista previa
en.LPreview=Preview
es.Activate=Usarlo como protector de pantalla tras
en.Activate=Use it as my screensaver after
es.Minutes=minutos
en.Minutes=minutes
es.RunTest=Probarlo ahora
en.RunTest=Try it now
es.RunSettings=Abrir la configuración de protector de pantalla de Windows
en.RunSettings=Open Windows screensaver settings
es.IconSettings=Configurar Screensaver FX
en.IconSettings=Configure Screensaver FX
es.IconTest=Probar Screensaver FX
en.IconTest=Try Screensaver FX
es.DeleteConfig=¿Borrar también tu configuración de Screensaver FX?
en.DeleteConfig=Also delete your Screensaver FX settings?
es.NeedTitle=Escribe un título o elige otro rótulo.
en.NeedTitle=Type a title or choose another banner.

[Files]
#if HaveArm64
Source: "{#Arm64Bin}"; DestDir: "{app}"; DestName: "ScreensaverFX.exe"; Check: IsArm64; Flags: ignoreversion
Source: "{#Arm64Bin}"; DestDir: "{app}"; DestName: "ScreensaverFX.scr"; Check: IsArm64; Flags: ignoreversion
Source: "{#Arm64Bin}"; DestDir: "{sys}"; DestName: "ScreensaverFX.scr"; Check: IsArm64 and IsAdminInstallMode; Flags: ignoreversion
Source: "{#Arm64Bin}"; DestName: "ssfx-arm64.exe"; Flags: dontcopy
#endif
Source: "{#X64Bin}"; DestDir: "{app}"; DestName: "ScreensaverFX.exe"; Check: UseX64; Flags: ignoreversion
Source: "{#X64Bin}"; DestDir: "{app}"; DestName: "ScreensaverFX.scr"; Check: UseX64; Flags: ignoreversion
Source: "{#X64Bin}"; DestDir: "{sys}"; DestName: "ScreensaverFX.scr"; Check: UseX64 and IsAdminInstallMode; Flags: ignoreversion
Source: "{#X64Bin}"; DestName: "ssfx-x64.exe"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\{cm:IconSettings}"; Filename: "{app}\ScreensaverFX.exe"; Parameters: "/c"
Name: "{autoprograms}\{cm:IconTest}"; Filename: "{app}\ScreensaverFX.exe"; Parameters: "/s"

[Registry]
; Vigilante: lanza el protector configurado aunque una app en primer plano lo bloquee.
; Para todos los usuarios en HKLM; solo para ti en HKCU.
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ScreensaverFX"; ValueData: """{app}\ScreensaverFX.exe"" /watch"; Flags: uninsdeletevalue

[UninstallDelete]
Type: filesandordirs; Name: "{app}\update"

[Run]
Filename: "{app}\ScreensaverFX.exe"; Parameters: "/watch"; Flags: nowait runasoriginaluser; Check: StartWatcher
Filename: "{app}\ScreensaverFX.exe"; Parameters: "/c"; Flags: nowait runasoriginaluser; Check: IsFromConfig
Filename: "{app}\ScreensaverFX.exe"; Parameters: "/s"; Description: "{cm:RunTest}"; Flags: postinstall nowait skipifsilent unchecked
Filename: "control.exe"; Parameters: "desk.cpl,,@screensaver"; Description: "{cm:RunSettings}"; Flags: postinstall nowait skipifsilent unchecked shellexec

[Code]
const
  SPI_SETSCREENSAVEACTIVE = 17;
  SPI_SETSCREENSAVETIMEOUT = 15;
  SPIF_UPDATE = 3; { SPIF_UPDATEINIFILE or SPIF_SENDCHANGE }
  DESKTOP_KEY = 'Control Panel\Desktop';
  PREVIEW_DELAY = 350;
  EVENT_MODIFY_STATE = 2;

type
  TTimerProc = procedure(H: LongWord; Msg: LongWord; IdEvent: LongWord; Time: LongWord);

var
  Page: TWizardPage;
  BannerCombo, ThemeCombo, FxCombo, SizeCombo: TNewComboBox;
  TitleEdit, SubtitleEdit, MinutesEdit: TNewEdit;
  TitleLabel, SubtitleLabel, PreviewLabel: TNewStaticText;
  ActivateCheck: TNewCheckBox;
  PreviewHost, Preview: TPanel;
  ThemeIds, FxValues: TStringList;
  CfgKeys, CfgVals: TStringList; { config.ini existente }
  HasArtFile: Boolean;
  PreviewExe: String;
  PreviewTimer: LongWord;

function SystemParametersInfo(uiAction, uiParam, pvParam, fWinIni: LongWord): BOOL;
  external 'SystemParametersInfoW@user32.dll stdcall';

{ /AUTOUPDATE: lo pasa el actualizador (windows/src/update.rs). No toca la configuración ni el
  protector elegido, y como SYSTEM no lanza el vigilante (lo relanza el actualizador en cada sesión). }
function IsAutoUpdate: Boolean;
begin
  Result := Pos('/AUTOUPDATE', Uppercase(GetCmdTail)) > 0;
end;

{ /FROMCONFIG: actualización pedida desde la configuración (windows/src/settings.rs). Al acabar
  vuelve a abrirla y, aunque sea para todos, arranca el vigilante de esta sesión (lo detuvo). }
function IsFromConfig: Boolean;
begin
  Result := Pos('/FROMCONFIG', Uppercase(GetCmdTail)) > 0;
end;

function StartWatcher: Boolean;
begin
  Result := IsFromConfig or not (IsAutoUpdate and IsAdminInstallMode);
end;

function GetLongPathName(lpszShortPath: String; lpszLongPath: String; cchBuffer: Cardinal): Cardinal;
  external 'GetLongPathNameW@kernel32.dll stdcall';

{ Ruta larga: Windows guarda a veces el protector como C:\WINDOWS\system32\SCREEN~1.SCR. }
function LongPath(const S: String): String;
var
  N: Cardinal;
begin
  Result := StringOfChar(' ', 1024);
  N := GetLongPathName(S, Result, 1024);
  if (N = 0) or (N >= 1024) then
    Result := S
  else
    SetLength(Result, N);
end;

function SetEnvironmentVariable(lpName, lpValue: String): BOOL;
  external 'SetEnvironmentVariableW@kernel32.dll stdcall';
function SetTimer(hWnd, nIDEvent, uElapse, lpTimerFunc: LongWord): LongWord;
  external 'SetTimer@user32.dll stdcall';
function KillTimer(hWnd, uIDEvent: LongWord): BOOL;
  external 'KillTimer@user32.dll stdcall';
function OpenEvent(dwDesiredAccess: LongWord; bInheritHandle: BOOL; lpName: String): LongWord;
  external 'OpenEventW@kernel32.dll stdcall';
function SetEvent(hEvent: LongWord): BOOL;
  external 'SetEvent@kernel32.dll stdcall';
function CloseHandle(hObject: LongWord): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';

function UseX64: Boolean;
begin
#if HaveArm64
  Result := not IsArm64;
#else
  Result := True;
#endif
end;

function DataDir: String;
begin
  Result := ExpandConstant('{userappdata}\ScreensaverFX');
end;

function ScrPath: String;
begin
  if IsAdminInstallMode then
    Result := ExpandConstant('{sys}\ScreensaverFX.scr')
  else
    Result := ExpandConstant('{app}\ScreensaverFX.scr');
end;

{ --- config.ini ------------------------------------------------------------ }

procedure LoadConfig;
var
  Lines: TArrayOfString;
  I, P, K: Integer;
  L: String;
begin
  CfgKeys := TStringList.Create;
  CfgVals := TStringList.Create;
  if LoadStringsFromFile(DataDir + '\config.ini', Lines) then
    for I := 0 to GetArrayLength(Lines) - 1 do
    begin
      L := Lines[I];
      if (I = 0) and (Length(L) > 0) and (Ord(L[1]) = $FEFF) then
        Delete(L, 1, 1);
      P := Pos('=', L);
      if (P > 1) and (Copy(Trim(L), 1, 1) <> '#') then
      begin
        { Clave repetida: gana la última, como en el protector (core/src/config.rs). }
        K := CfgKeys.IndexOf(Lowercase(Trim(Copy(L, 1, P - 1))));
        if K >= 0 then
          CfgVals[K] := Trim(Copy(L, P + 1, MaxInt))
        else
        begin
          CfgKeys.Add(Lowercase(Trim(Copy(L, 1, P - 1))));
          CfgVals.Add(Trim(Copy(L, P + 1, MaxInt)));
        end;
      end;
    end;
end;

function Cv(Key, Def: String): String;
var
  I: Integer;
begin
  Result := '';
  I := CfgKeys.IndexOf(Key);
  if I >= 0 then
    Result := CfgVals[I];
  if Result = '' then
    Result := Def;
end;

function BannerValue: String;
begin
  case BannerCombo.ItemIndex of
    1: Result := 'text';
    2: Result := 'file';
  else
    Result := 'default';
  end;
end;

function SizeValue: String;
begin
  case SizeCombo.ItemIndex of
    0: Result := 'S';
    2: Result := 'L';
  else
    Result := 'M';
  end;
end;

procedure AddLine(var A: TArrayOfString; S: String);
var
  N: Integer;
begin
  N := GetArrayLength(A);
  SetArrayLength(A, N + 1);
  A[N] := S;
end;

function BuildIni: TArrayOfString;
var
  A: TArrayOfString;
  I: Integer;
  K: String;
begin
  AddLine(A, '# Screensaver FX');
  AddLine(A, 'banner=' + BannerValue);
  AddLine(A, 'title=' + TitleEdit.Text);
  AddLine(A, 'subtitle=' + SubtitleEdit.Text);
  AddLine(A, 'theme=' + ThemeIds[ThemeCombo.ItemIndex]);
  AddLine(A, 'size=' + SizeValue);
  AddLine(A, 'effects=' + FxValues[FxCombo.ItemIndex]);
  { Lo que el asistente no pregunta (idioma, letra, pantallas...) se conserva tal cual. }
  for I := 0 to CfgKeys.Count - 1 do
  begin
    K := CfgKeys[I];
    if (K <> 'banner') and (K <> 'title') and (K <> 'subtitle') and (K <> 'theme') and (K <> 'size') and (K <> 'effects') and (CfgVals[I] <> '') then
      AddLine(A, K + '=' + CfgVals[I]);
  end;
  Result := A;
end;

{ --- Vista previa en vivo -------------------------------------------------- }
{ El propio .scr se ejecuta en modo /p dentro de un panel del asistente.     }
{ Para reiniciarla se destruye el panel: el proceso detecta que su ventana  }
{ padre ya no existe y termina solo.                                          }

procedure StopPreview;
begin
  if Preview <> nil then
  begin
    Preview.Free;
    Preview := nil;
  end;
end;

procedure StartPreview;
var
  Ini: String;
  Rc: Integer;
begin
  StopPreview;
  if PreviewExe = '' then
    Exit;
  Preview := TPanel.Create(PreviewHost);
  Preview.Parent := PreviewHost;
  Preview.BevelOuter := bvNone;
  Preview.Color := $1A1311;
  Preview.Left := ScaleX(1);
  Preview.Top := ScaleY(1);
  Preview.Width := PreviewHost.ClientWidth - ScaleX(2);
  Preview.Height := PreviewHost.ClientHeight - ScaleY(2);
  Ini := ExpandConstant('{tmp}\preview.ini');
  SaveStringsToUTF8File(Ini, BuildIni, False);
  SetEnvironmentVariable('SSFX_CONFIG', Ini);
  Exec(PreviewExe, '/p ' + IntToStr(Preview.Handle), '', SW_SHOWNORMAL, ewNoWait, Rc);
  SetEnvironmentVariable('SSFX_CONFIG', '');
end;

procedure PreviewTick(H: LongWord; Msg: LongWord; IdEvent: LongWord; Time: LongWord);
begin
  KillTimer(0, PreviewTimer);
  PreviewTimer := 0;
  if WizardForm.CurPageID = Page.ID then
    StartPreview;
end;

procedure SchedulePreview;
begin
  if PreviewTimer <> 0 then
    KillTimer(0, PreviewTimer);
  PreviewTimer := SetTimer(0, 0, PREVIEW_DELAY, CreateCallback(@PreviewTick));
end;

{ --- Página de personalización -------------------------------------------- }

procedure SyncEnabled;
var
  Txt: Boolean;
begin
  Txt := BannerCombo.ItemIndex = 1;
  TitleEdit.Enabled := Txt;
  SubtitleEdit.Enabled := Txt;
  TitleLabel.Enabled := Txt;
  SubtitleLabel.Enabled := Txt;
  MinutesEdit.Enabled := ActivateCheck.Checked;
end;

procedure OnChange(Sender: TObject);
begin
  SyncEnabled;
  SchedulePreview;
end;

procedure OnActivateClick(Sender: TObject);
begin
  SyncEnabled;
end;

function AddLabel(Caption: String; Top, Left: Integer): TNewStaticText;
begin
  Result := TNewStaticText.Create(Page);
  Result.Parent := Page.Surface;
  Result.Caption := Caption;
  Result.Left := Left;
  Result.Top := Top;
end;

function AddCombo(Top, Left, Width: Integer): TNewComboBox;
begin
  Result := TNewComboBox.Create(Page);
  Result.Parent := Page.Surface;
  Result.Style := csDropDownList;
  Result.Left := Left;
  Result.Top := Top;
  Result.Width := Width;
  Result.OnChange := @OnChange;
end;

function AddEdit(Top, Left, Width, MaxLen: Integer; Text: String): TNewEdit;
begin
  Result := TNewEdit.Create(Page);
  Result.Parent := Page.Surface;
  Result.Left := Left;
  Result.Top := Top;
  Result.Width := Width;
  Result.MaxLength := MaxLen;
  Result.Text := Text;
  Result.OnChange := @OnChange;
end;

procedure AddTheme(Id, Name: String);
begin
  ThemeIds.Add(Id);
  ThemeCombo.Items.Add(Name);
end;

procedure AddFx(Value, Name: String);
begin
  FxValues.Add(Value);
  FxCombo.Items.Add(Name);
end;

procedure CreateCustomizePage;
var
  ColW, Gap, Y, RowH, LblH, PrevW, PrevX, I, Mins: Integer;
  Theme, Fx, Banner: String;
begin
  Page := CreateCustomPage(wpWelcome, CustomMessage('PageTitle'), CustomMessage('PageDesc'));
  Gap := ScaleX(16);
  ColW := (Page.SurfaceWidth - Gap) * 9 div 20;
  PrevX := ColW + Gap;
  PrevW := Page.SurfaceWidth - PrevX;
  LblH := ScaleY(17);
  RowH := ScaleY(46);
  Y := 0;

  AddLabel(CustomMessage('LBanner'), Y, 0);
  BannerCombo := AddCombo(Y + LblH, 0, ColW);
  BannerCombo.Items.Add(CustomMessage('BannerDefault'));
  BannerCombo.Items.Add(CustomMessage('BannerText'));
  HasArtFile := FileExists(DataDir + '\screensaver.txt');
  if HasArtFile then
    BannerCombo.Items.Add(CustomMessage('BannerFile'));
  Banner := Lowercase(Cv('banner', 'default'));
  if Banner = 'text' then
    BannerCombo.ItemIndex := 1
  else if (Banner = 'file') and HasArtFile then
    BannerCombo.ItemIndex := 2
  else
    BannerCombo.ItemIndex := 0;
  Y := Y + RowH;

  TitleLabel := AddLabel(CustomMessage('LTitle'), Y, 0);
  TitleEdit := AddEdit(Y + LblH, 0, ColW, 16, Cv('title', 'Hola'));
  Y := Y + RowH;
  SubtitleLabel := AddLabel(CustomMessage('LSubtitle'), Y, 0);
  SubtitleEdit := AddEdit(Y + LblH, 0, ColW, 32, Cv('subtitle', ''));
  Y := Y + RowH;

  AddLabel(CustomMessage('LTheme'), Y, 0);
  ThemeCombo := AddCombo(Y + LblH, 0, ColW);
  ThemeIds := TStringList.Create;
  AddTheme('auto', CustomMessage('ThemeAuto'));
  AddTheme('random', CustomMessage('ThemeRandom'));
  AddTheme('aero', 'Aero');
  AddTheme('aero-night', 'Aero Night');
  AddTheme('obsidiana', 'Obsidiana');
  AddTheme('fluent', 'Fluent');
  AddTheme('fluent-light', 'Fluent Light');
  AddTheme('tahoe', 'Tahoe');
  AddTheme('ventura', 'Ventura');
  AddTheme('big-sur', 'Big Sur');
  AddTheme('graphite', 'Graphite');
  AddTheme('cupertino', 'Cupertino');
  AddTheme('cupertino-light', 'Cupertino Light');
  Theme := Lowercase(Cv('theme', 'auto'));
  if Theme = 'system' then
    Theme := 'auto';
  ThemeCombo.ItemIndex := 0;
  for I := 0 to ThemeIds.Count - 1 do
    if ThemeIds[I] = Theme then
      ThemeCombo.ItemIndex := I;
  Y := Y + RowH;

  AddLabel(CustomMessage('LEffects'), Y, 0);
  FxCombo := AddCombo(Y + LblH, 0, ColW);
  FxValues := TStringList.Create;
  AddFx('all', CustomMessage('FxAll'));
  AddFx('classic', CustomMessage('FxClassic'));
  AddFx('modern', CustomMessage('FxModern'));
  Fx := Lowercase(Cv('effects', 'all'));
  FxCombo.ItemIndex := FxValues.IndexOf(Fx);
  if FxCombo.ItemIndex < 0 then
  begin
    AddFx(Fx, CustomMessage('FxCustom') + ': ' + Fx);
    FxCombo.ItemIndex := FxValues.Count - 1;
  end;

  { Columna derecha: vista previa 16:9 y tamaño. }
  PreviewLabel := AddLabel(CustomMessage('LPreview'), 0, PrevX);
  PreviewHost := TPanel.Create(Page);
  PreviewHost.Parent := Page.Surface;
  PreviewHost.BevelOuter := bvNone;
  PreviewHost.Color := $3A2B24;
  PreviewHost.Left := PrevX;
  PreviewHost.Top := LblH;
  PreviewHost.Width := PrevW;
  PreviewHost.Height := PrevW * 9 div 16;

  Y := PreviewHost.Top + PreviewHost.Height + ScaleY(12);
  AddLabel(CustomMessage('LSize'), Y, PrevX);
  SizeCombo := AddCombo(Y + LblH, PrevX, PrevW);
  SizeCombo.Items.Add(CustomMessage('SizeS'));
  SizeCombo.Items.Add(CustomMessage('SizeM'));
  SizeCombo.Items.Add(CustomMessage('SizeL'));
  case Uppercase(Cv('size', 'M')) of
    'S': SizeCombo.ItemIndex := 0;
    'L': SizeCombo.ItemIndex := 2;
  else
    SizeCombo.ItemIndex := 1;
  end;

  { Fila inferior: activar como protector de pantalla del sistema. }
  Y := FxCombo.Top + FxCombo.Height + ScaleY(18);
  ActivateCheck := TNewCheckBox.Create(Page);
  ActivateCheck.Parent := Page.Surface;
  ActivateCheck.Caption := CustomMessage('Activate');
  ActivateCheck.Top := Y;
  ActivateCheck.Width := ScaleX(250);
  ActivateCheck.Checked := True;
  ActivateCheck.OnClick := @OnActivateClick;
  if RegQueryStringValue(HKCU, DESKTOP_KEY, 'ScreenSaveTimeOut', Fx) then
    Mins := StrToIntDef(Fx, 300) div 60;
  if Mins < 1 then
    Mins := 5;
  MinutesEdit := AddEdit(Y - ScaleY(3), ActivateCheck.Left + ActivateCheck.Width + ScaleX(4), ScaleX(36), 3, IntToStr(Mins));
  MinutesEdit.OnChange := nil;
  AddLabel(CustomMessage('Minutes'), Y, MinutesEdit.Left + MinutesEdit.Width + ScaleX(6));

  SyncEnabled;
end;

{ --- Eventos del asistente ------------------------------------------------- }

procedure InitializeWizard;
begin
  LoadConfig;
  CreateCustomizePage;
  { /NOACTIVATE: instala sin cambiar el protector de pantalla actual (útil con /VERYSILENT). }
  if (Pos('/NOACTIVATE', Uppercase(GetCmdTail)) > 0) or IsAutoUpdate then
    ActivateCheck.Checked := False;
  try
    if UseX64 then
    begin
      ExtractTemporaryFile('ssfx-x64.exe');
      PreviewExe := ExpandConstant('{tmp}\ssfx-x64.exe');
    end
    else
    begin
      ExtractTemporaryFile('ssfx-arm64.exe');
      PreviewExe := ExpandConstant('{tmp}\ssfx-arm64.exe');
    end;
  except
    PreviewExe := '';
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = Page.ID then
    StartPreview
  else
    StopPreview;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = Page.ID) and (BannerCombo.ItemIndex = 1) and
     (Trim(TitleEdit.Text) = '') and (Trim(SubtitleEdit.Text) = '') then
  begin
    MsgBox(CustomMessage('NeedTitle'), mbInformation, MB_OK);
    Result := False;
  end;
end;

{ Detiene el vigilante de esta sesión (evento que espera, ver windows/src/watch.rs) para poder
  reemplazar o borrar el ejecutable. No ejecuta el .exe instalado: uno antiguo no conoce /unwatch. }
procedure StopWatcher;
var
  H: LongWord;
begin
  H := OpenEvent(EVENT_MODIFY_STATE, False, 'Local\ScreensaverFX.Watch.Quit');
  if H <> 0 then
  begin
    SetEvent(H);
    CloseHandle(H);
    Sleep(500);
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  StopWatcher;
end;

{ Para todos: tarea programada que busca actualizaciones como SYSTEM al arrancar y una vez al día. }
procedure RegisterUpdateTask;
var
  Lines: TArrayOfString;
  F: String;
  Code: Integer;
begin
  F := ExpandConstant('{tmp}\update-task.xml');
  SetArrayLength(Lines, 1);
  { Sin declaración <?xml?> y en ANSI: schtasks /XML rechaza el UTF-8 declarado («no se pudo
    cambiar la codificación») y también el BOM. }
  Lines[0] :=
    '<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">' + #13#10 +
    '  <RegistrationInfo><Description>Busca e instala actualizaciones de Screensaver FX.</Description></RegistrationInfo>' + #13#10 +
    '  <Triggers>' + #13#10 +
    '    <BootTrigger><Delay>PT10M</Delay></BootTrigger>' + #13#10 +
    '    <CalendarTrigger><StartBoundary>2026-01-01T12:00:00</StartBoundary><RandomDelay>PT2H</RandomDelay>' +
    '<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay></CalendarTrigger>' + #13#10 +
    '  </Triggers>' + #13#10 +
    '  <Principals><Principal id="System"><UserId>S-1-5-18</UserId><RunLevel>HighestAvailable</RunLevel></Principal></Principals>' + #13#10 +
    '  <Settings>' + #13#10 +
    '    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>' + #13#10 +
    '    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>' + #13#10 +
    '    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>' + #13#10 +
    '    <StartWhenAvailable>true</StartWhenAvailable>' + #13#10 +
    '    <RunOnlyIfNetworkAvailable>true</RunOnlyIfNetworkAvailable>' + #13#10 +
    '    <ExecutionTimeLimit>PT30M</ExecutionTimeLimit>' + #13#10 +
    '    <Priority>7</Priority>' + #13#10 +
    '  </Settings>' + #13#10 +
    '  <Actions Context="System"><Exec><Command>"' + ExpandConstant('{app}\ScreensaverFX.exe') + '"</Command>' +
    '<Arguments>/update</Arguments></Exec></Actions>' + #13#10 +
    '</Task>';
  SaveStringsToFile(F, Lines, False);
  Exec(ExpandConstant('{sys}\schtasks.exe'), '/Create /F /TN "ScreensaverFX\Update" /XML "' + F + '"', '', SW_HIDE, ewWaitUntilTerminated, Code);
  if Code <> 0 then
    Log(Format('schtasks /Create terminó con %d', [Code]));
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Mins: Integer;
begin
  if CurStep <> ssPostInstall then
    Exit;
  if IsAdminInstallMode then
    RegisterUpdateTask;
  if IsAutoUpdate then
    Exit;
  ForceDirectories(DataDir);
  SaveStringsToUTF8File(DataDir + '\config.ini', BuildIni, False);
  if ActivateCheck.Checked then
  begin
    Mins := StrToIntDef(MinutesEdit.Text, 5);
    if Mins < 1 then
      Mins := 1;
    RegWriteStringValue(HKCU, DESKTOP_KEY, 'SCRNSAVE.EXE', ScrPath);
    SystemParametersInfo(SPI_SETSCREENSAVETIMEOUT, Mins * 60, 0, SPIF_UPDATE);
    SystemParametersInfo(SPI_SETSCREENSAVEACTIVE, 1, 0, SPIF_UPDATE);
  end;
end;

procedure DeinitializeSetup;
begin
  StopPreview;
end;

{ --- Desinstalación -------------------------------------------------------- }

{ Carpeta de la otra instalación (para todos / solo para mí), si la hay. }
function OtherInstallDir: String;
var
  Root: Integer;
begin
  Result := '';
  if IsAdminInstallMode then
    Root := HKCU
  else
    Root := HKLM;
  if RegQueryStringValue(Root, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{6B0E3F2A-5C1D-4E8B-9A7F-3D2C1B0A9E8F}_is1',
     'InstallLocation', Result) and FileExists(AddBackslash(Result) + 'ScreensaverFX.exe') then
    Result := AddBackslash(Result)
  else
    Result := '';
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Cur, Other: String;
  Code: Integer;
begin
  Other := OtherInstallDir;
  if CurUninstallStep = usUninstall then
  begin
    if IsAdminInstallMode then
      Exec(ExpandConstant('{sys}\schtasks.exe'), '/Delete /F /TN "ScreensaverFX\Update"', '', SW_HIDE, ewWaitUntilTerminated, Code);
    StopWatcher;
    { Solo si apunta a este protector: no desactiva el de la otra instalación. }
    if RegQueryStringValue(HKCU, DESKTOP_KEY, 'SCRNSAVE.EXE', Cur) then
      Cur := LongPath(Cur);
    if (Cur <> '') and
       ((CompareText(Cur, LongPath(ExpandConstant('{app}\ScreensaverFX.scr'))) = 0) or
        (IsAdminInstallMode and (CompareText(Cur, LongPath(ExpandConstant('{sys}\ScreensaverFX.scr'))) = 0)) or
        ((Other = '') and (Pos('screensaverfx', Lowercase(Cur)) > 0))) then
    begin
      RegDeleteValue(HKCU, DESKTOP_KEY, 'SCRNSAVE.EXE');
      SystemParametersInfo(SPI_SETSCREENSAVEACTIVE, 0, 0, SPIF_UPDATE);
    end;
  end;
  if CurUninstallStep <> usPostUninstall then
    Exit;
  if Other <> '' then
  begin
    { Queda la otra instalación: su vigilante vuelve a correr (sin elevar) y la configuración se queda. }
    if not IsAdminInstallMode then
      Exec(Other + 'ScreensaverFX.exe', '/watch', '', SW_HIDE, ewNoWait, Code);
    Exit;
  end;
  if DirExists(ExpandConstant('{userappdata}\ScreensaverFX')) then
    if SuppressibleMsgBox(CustomMessage('DeleteConfig'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2, IDNO) = IDYES then
      DelTree(ExpandConstant('{userappdata}\ScreensaverFX'), True, True, True);
end;
