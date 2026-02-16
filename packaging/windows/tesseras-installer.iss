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
SetupIconFile=C:\tesseras\apps\flutter\windows\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\tesseras_app.exe
WizardStyle=modern
WizardImageFile=C:\tesseras\packaging\windows\wizard_image.bmp
WizardSmallImageFile=C:\tesseras\packaging\windows\wizard_small_image.bmp
LicenseFile=C:\tesseras\packaging\windows\LICENSE.txt
PrivilegesRequired=lowest

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
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "C:\tesseras\apps\flutter\build\windows\x64\runner\Release\tesseras_app.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\tesseras\apps\flutter\build\windows\x64\runner\Release\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\tesseras\apps\flutter\build\windows\x64\runner\Release\data\*"; DestDir: "{app}\data"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Tesseras"; Filename: "{app}\tesseras_app.exe"
Name: "{group}\{cm:UninstallProgram,Tesseras}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Tesseras"; Filename: "{app}\tesseras_app.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\tesseras_app.exe"; Description: "{cm:LaunchProgram,Tesseras}"; Flags: nowait postinstall skipifsilent
