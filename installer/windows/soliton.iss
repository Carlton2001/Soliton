; Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0).
#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef PayloadDir
  #error PayloadDir is required
#endif
[Setup]
AppId=Soliton.Windows
AppName=Soliton
AppVersion={#AppVersion}
AppPublisher=Soliton
DefaultDirName={localappdata}\Programs\Soliton
DefaultGroupName=Soliton
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
UninstallDisplayIcon={app}\Soliton.exe
OutputBaseFilename=Soliton_{#AppVersion}_x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
[Files]
Source: "{#PayloadDir}\Soliton.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#PayloadDir}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{autoprograms}\Soliton"; Filename: "{app}\Soliton.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\Soliton"; Filename: "{app}\Soliton.exe"; WorkingDir: "{app}"; Tasks: desktopicon
[Run]
Filename: "{app}\Soliton.exe"; Description: "{cm:LaunchProgram,Soliton}"; Flags: nowait postinstall skipifsilent; Check: not IsUpdate
Filename: "{app}\Soliton.exe"; Flags: nowait; Check: IsUpdate
; User data in LocalAppData\Soliton is deliberately preserved on uninstall.

[Code]
function IsUpdate: Boolean;
var
  Index: Integer;
begin
  Result := False;
  for Index := 1 to ParamCount do
    if CompareText(ParamStr(Index), '/UPDATE') = 0 then Result := True;
end;

function OpenProcess(Access: DWORD; Inherit: BOOL; ProcessId: DWORD): THandle;
  external 'OpenProcess@kernel32.dll stdcall';
function WaitForSingleObject(Handle: THandle; Milliseconds: DWORD): DWORD;
  external 'WaitForSingleObject@kernel32.dll stdcall';
function CloseHandle(Handle: THandle): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';

function InitializeSetup: Boolean;
var
  Process: THandle;
begin
  Result := True;
  Process := OpenProcess($00100000, False, StrToIntDef(ExpandConstant('{param:WAITPID|0}'), 0));
  if Process <> 0 then
  begin
    // Never replace files when Soliton has not finished saving and shutting down.
    Result := WaitForSingleObject(Process, 60000) = 0;
    CloseHandle(Process);
  end;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := IsUpdate;
end;
