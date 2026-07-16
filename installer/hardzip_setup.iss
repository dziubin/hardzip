; HardZIP Installer Script for Inno Setup
; Download Inno Setup: https://jrsoftware.org/isdl.php
; Open this file in Inno Setup Compiler and click Build

[Setup]
AppName=HardZIP
AppVersion=0.2.1
AppVerName=HardZIP 0.2.1 Beta
AppPublisher=Łukasz Dziubiński
AppPublisherURL=https://www.ydi.pl
AppSupportURL=https://www.ydi.pl
AppUpdatesURL=https://www.ydi.pl
DefaultDirName={autopf}\HardZIP
DefaultGroupName=HardZIP
AllowNoIcons=yes
OutputDir=output
OutputBaseFilename=HardZIP_0.2.1_Setup
SetupIconFile=..\assets\hardzip.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UninstallDisplayIcon={app}\hardzip.exe
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=license.txt
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; OnlyBelowVersion: 6.1; Check: not IsAdminInstallMode
Name: "contextmenu"; Description: "Add HardZIP to right-click context menu"; GroupDescription: "System integration:"
Name: "fileassoc"; Description: "Associate archive files (.hza, .zip, .7z, .tar, .gz, .bz2, .xz)"; GroupDescription: "System integration:"

[Files]
Source: "..\target\release\hardzip.exe"; DestDir: "{app}"; Flags: ignoreversion
; Uncomment if you have the icon file:
; Source: "..\assets\hardzip.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\HardZIP"; Filename: "{app}\hardzip.exe"
Name: "{group}\{cm:UninstallProgram,HardZIP}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\HardZIP"; Filename: "{app}\hardzip.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\hardzip.exe"; Description: "{cm:LaunchProgram,HardZIP}"; Flags: nowait postinstall skipifsilent

[Registry]
; File association for .hza
Root: HKA; Subkey: "Software\Classes\.hza"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive"; ValueType: string; ValueName: ""; ValueData: "HardZIP Archive"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\hardzip.exe,0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\hardzip.exe"" ""%1"""; Tasks: fileassoc

; Additional file associations
Root: HKA; Subkey: "Software\Classes\.zip"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.7z"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.tar"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.gz"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.tgz"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.bz2"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.xz"; ValueType: string; ValueName: ""; ValueData: "HardZIP.Archive"; Flags: uninsdeletevalue; Tasks: fileassoc

; Context menu - Compress
Root: HKA; Subkey: "Software\Classes\*\shell\HardZIP_Compress"; ValueType: string; ValueName: ""; ValueData: "Compress with HardZIP"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\*\shell\HardZIP_Compress"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\hardzip.exe"; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\*\shell\HardZIP_Compress\command"; ValueType: string; ValueName: ""; ValueData: """{app}\hardzip.exe"" compress ""%1"" -o ""%1.hza"""; Tasks: contextmenu

; Context menu - Extract (for archive files)
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive\shell\HardZIP_Extract"; ValueType: string; ValueName: ""; ValueData: "Extract with HardZIP"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive\shell\HardZIP_Extract"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\hardzip.exe"; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\HardZIP.Archive\shell\HardZIP_Extract\command"; ValueType: string; ValueName: ""; ValueData: """{app}\hardzip.exe"" extract ""%1"""; Tasks: contextmenu

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
