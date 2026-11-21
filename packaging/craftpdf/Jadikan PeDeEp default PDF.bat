@echo off
setlocal
rem Daftarkan PeDeEp (folder ini) sebagai aplikasi PDF untuk user ini saja (HKCU), tanpa admin.
rem Kalau folder PeDeEp dipindah, jalankan file ini lagi dari tempat yang baru.
set "EXE=%~dp0PeDeEp.exe"
if not exist "%EXE%" (
  echo PeDeEp.exe tidak ditemukan di "%~dp0".
  echo Taruh file .bat ini di dalam folder PeDeEp, di samping PeDeEp.exe.
  pause
  exit /b 1
)

set "C=HKCU\Software\Classes"
set "CAP=HKCU\Software\PeDeEp.Portable\Capabilities"
reg add "%C%\PeDeEp.Portable" /ve /d "PDF Document" /f >nul || goto gagal
reg add "%C%\PeDeEp.Portable\DefaultIcon" /ve /d "\"%EXE%\",0" /f >nul || goto gagal
reg add "%C%\PeDeEp.Portable\shell\open\command" /ve /d "\"%EXE%\" \"%%1\"" /f >nul || goto gagal
reg add "%C%\.pdf\OpenWithProgids" /v PeDeEp.Portable /d "" /f >nul || goto gagal
reg add "%CAP%" /v ApplicationName /d "PeDeEp" /f >nul || goto gagal
reg add "%CAP%" /v ApplicationDescription /d "PDF editor (based on PdfCraft)" /f >nul || goto gagal
reg add "%CAP%\FileAssociations" /v .pdf /d PeDeEp.Portable /f >nul || goto gagal
reg add "HKCU\Software\RegisteredApplications" /v PeDeEp /d "Software\PeDeEp.Portable\Capabilities" /f >nul || goto gagal

rem Bersihkan pendaftaran lama dengan nama CraftPdf, kalau ada.
reg delete "HKCU\Software\Classes\CraftPdf.Portable" /f >nul 2>&1
reg delete "HKCU\Software\CraftPdf.Portable" /f >nul 2>&1
reg delete "HKCU\Software\Classes\.pdf\OpenWithProgids" /v CraftPdf.Portable /f >nul 2>&1
reg delete "HKCU\Software\RegisteredApplications" /v CraftPdf /f >nul 2>&1

echo PeDeEp sudah terdaftar.
echo.
echo Langkah terakhir (Windows tidak mengizinkan script melakukannya):
echo Di jendela Settings yang terbuka, klik ".pdf" lalu pilih PeDeEp.
echo.
start "" "ms-settings:defaultapps?registeredAppUser=PeDeEp"
pause
exit /b 0

:gagal
echo Gagal menulis ke registry.
pause
exit /b 1
