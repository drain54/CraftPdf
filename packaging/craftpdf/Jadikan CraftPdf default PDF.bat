@echo off
setlocal
rem Daftarkan CraftPdf (folder ini) sebagai aplikasi PDF untuk user ini saja (HKCU), tanpa admin.
rem Kalau folder CraftPdf dipindah, jalankan file ini lagi dari tempat yang baru.
set "EXE=%~dp0pdfcraft.exe"
if not exist "%EXE%" (
  echo pdfcraft.exe tidak ditemukan di "%~dp0".
  echo Taruh file .bat ini di dalam folder CraftPdf, di samping pdfcraft.exe.
  pause
  exit /b 1
)

set "C=HKCU\Software\Classes"
set "CAP=HKCU\Software\CraftPdf.Portable\Capabilities"
reg add "%C%\CraftPdf.Portable" /ve /d "PDF Document" /f >nul || goto gagal
reg add "%C%\CraftPdf.Portable\DefaultIcon" /ve /d "\"%EXE%\",0" /f >nul || goto gagal
reg add "%C%\CraftPdf.Portable\shell\open\command" /ve /d "\"%EXE%\" \"%%1\"" /f >nul || goto gagal
reg add "%C%\.pdf\OpenWithProgids" /v CraftPdf.Portable /d "" /f >nul || goto gagal
reg add "%CAP%" /v ApplicationName /d "CraftPdf" /f >nul || goto gagal
reg add "%CAP%" /v ApplicationDescription /d "PDF editor (based on PdfCraft)" /f >nul || goto gagal
reg add "%CAP%\FileAssociations" /v .pdf /d CraftPdf.Portable /f >nul || goto gagal
reg add "HKCU\Software\RegisteredApplications" /v CraftPdf /d "Software\CraftPdf.Portable\Capabilities" /f >nul || goto gagal

echo CraftPdf sudah terdaftar.
echo.
echo Langkah terakhir (Windows tidak mengizinkan script melakukannya):
echo Di jendela Settings yang terbuka, klik ".pdf" lalu pilih CraftPdf.
echo.
start "" "ms-settings:defaultapps?registeredAppUser=CraftPdf"
pause
exit /b 0

:gagal
echo Gagal menulis ke registry.
pause
exit /b 1
