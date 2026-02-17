[Setup]
AppName=Tesseras
AppVersion=0.1.0
AppPublisher=Tesseras Project
AppPublisherURL=https://tesseras.org
DefaultDirName={autopf}\Tesseras
DefaultGroupName=Tesseras
OutputDir=C:\tesseras-installer
OutputBaseFilename=tesseras-setup-0.1.0-win64
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\tes.exe
WizardStyle=modern
WizardImageFile=C:\tesseras\packaging\windows\wizard_image.bmp
WizardSmallImageFile=C:\tesseras\packaging\windows\wizard_small_image.bmp
LicenseFile=C:\tesseras\packaging\windows\LICENSE.txt
PrivilegesRequired=lowest
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Messages]
english.WelcomeLabel1=Welcome to Tesseras
english.WelcomeLabel2=Preserve your memories across millennia.%n%nTesseras is a peer-to-peer network where your photos, audio, and stories are preserved forever — no company, no cloud, no single point of failure.%n%nThis will install Tesseras on your computer.
english.FinishedHeadingLabel=Tesseras is ready!
english.FinishedLabel=Tesseras has been installed on your computer.%n%nYour memories deserve to last forever. Welcome to the network.%n%n%nDedicated to Aninha, my great love — the reason I believe some things should last forever.

portuguese.WelcomeLabel1=Bem-vindo ao Tesseras
portuguese.WelcomeLabel2=Preserve suas memorias atraves dos milenios.%n%nTesseras e uma rede peer-to-peer onde suas fotos, audios e historias sao preservadas para sempre — sem empresa, sem nuvem, sem ponto unico de falha.%n%nIsso ira instalar o Tesseras no seu computador.
portuguese.FinishedHeadingLabel=Tesseras esta pronto!
portuguese.FinishedLabel=Tesseras foi instalado no seu computador.%n%nSuas memorias merecem durar para sempre. Bem-vindo a rede.%n%n%nDedicado a Aninha, meu grande amor — a razao pela qual eu acredito que algumas coisas devem durar para sempre.

[Tasks]
Name: "addtopath"; Description: "Add tes to PATH"; GroupDescription: "System integration:"

[Files]
Source: "C:\tesseras\target\release\tes.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Tesseras CLI"; Filename: "{cmd}"; Parameters: "/k ""{app}\tes.exe"" --help"; WorkingDir: "{userdocs}"
Name: "{group}\{cm:UninstallProgram,Tesseras}"; Filename: "{uninstallexe}"

[Registry]
; Add to user PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;
