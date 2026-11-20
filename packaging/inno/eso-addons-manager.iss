#ifndef MyAppVersion
  #define MyAppVersion "0.0.1"
#endif

#define MyAppName "ESO Addons Manager"
#define MyAppPublisher "powerbq"
#define MyAppURL "https://github.com/powerbq/eso-addons-manager"
#define MyAppExeName "eso-addons-manager.exe"

[Setup]
AppId={{4B4C64BD-E463-4DE4-B636-D91AA3C9CB9B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=auto
LicenseFile=..\..\LICENSE
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
OutputDir=..\..\target\dist\inno
OutputBaseFilename=eso-addons-manager-{#MyAppVersion}-windows-amd64-setup
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\packaging\app.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DataDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    DataDir := ExpandConstant('{localappdata}\EsoAddonsManager');
    if DirExists(DataDir) then
    begin
      if not UninstallSilent then
      begin
        if MsgBox('Also delete your settings, cache and custom add-ons?' + #13#10 + DataDir,
          mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
          DelTree(DataDir, True, True, True);
      end;
    end;
  end;
end;
