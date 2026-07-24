#define MyAppName "Black Hole Trash"
#define MyAppVersion "1.3.0"
#define MyAppPublisher "rrrjqy66"
#define MyAppURL "https://github.com/rrrjqy66/BlackHoleTrash"
#define MyAppExeName "BlackHoleTrash.exe"

[Setup]
AppId={{E34B31B5-ABB8-4D90-96C6-472685BF40AB}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\BlackHoleTrash
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
OutputDir=..\dist
OutputBaseFilename=BlackHoleTrash-Setup-x64
SetupIconFile=..\assets\black-hole-trash.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
LicenseFile=..\LICENSE
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=force
RestartApplications=no
AppMutex=Local\BlackHoleTrashOverlay
VersionInfoVersion=1.3.0.0
VersionInfoDescription={#MyAppName} Installer
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional options:"; Flags: unchecked

[Files]
Source: "..\target\release\BlackHoleTrash.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.en.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\community\qq-group.jpg"; DestDir: "{app}\assets\community"; Flags: ignoreversion
Source: "..\assets\community\donate.jpg"; DestDir: "{app}\assets\community"; Flags: ignoreversion
Source: "..\assets\community\star-history.svg"; DestDir: "{app}\assets\community"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Black Hole Trash"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\Black Hole Trash"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch Black Hole Trash"; Flags: nowait postinstall skipifsilent
