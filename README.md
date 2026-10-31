![Скриншот 3](3.png)

![Главная](Home.png)

![Скриншот 2](2.png)

![Скриншот 4](4.png)

## Установка

```powershell
Invoke-WebRequest -Uri "https://github.com/ScrSrc/rust-explorer-eleven/releases/latest/download/win11_explorer-1.0.0-x86_64.msi" -OutFile "win11_explorer.msi"
Start-Process msiexec.exe -ArgumentList "/i win11_explorer.msi" -Wait
