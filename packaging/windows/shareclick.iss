; Inno Setup script for ShareClick — produces a one-click Windows installer.
;
; Build (on Windows, after `cargo build --release --features tray`):
;   iscc /DMyAppVersion=0.1.0 packaging\windows\shareclick.iss
;
; Output: packaging\windows\Output\ShareClick-Setup-<version>.exe

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppName "ShareClick"
#define MyAppPublisher "ShareClick"
#define MyAppExeName "shareclick.exe"
#define MyAppURL "https://github.com/phun333/ShareClick"

[Setup]
AppId={{9F5B2C31-4E8A-4C1F-9A2D-7B3E6C1A0D42}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; Install for the current user only → no admin prompt (nicer UX).
PrivilegesRequired=lowest
OutputDir=Output
OutputBaseFilename=ShareClick-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=shareclick.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"
Name: "startupicon"; Description: "Start ShareClick automatically when I log in"; GroupDescription: "Startup:"; Flags: unchecked

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName} now"; Flags: nowait postinstall skipifsilent
