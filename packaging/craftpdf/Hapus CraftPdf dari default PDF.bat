@echo off
setlocal
rem Membatalkan "Jadikan CraftPdf default PDF.bat". Pilih aplikasi PDF lain dulu di Settings,
rem supaya Windows tidak menunjuk ke entri yang sudah dihapus.
echo Pilih dulu aplikasi PDF lain di Settings (klik ".pdf"), lalu kembali ke sini.
start "" "ms-settings:defaultapps"
pause
reg delete "HKCU\Software\Classes\CraftPdf.Portable" /f >nul 2>&1
reg delete "HKCU\Software\CraftPdf.Portable" /f >nul 2>&1
reg delete "HKCU\Software\Classes\.pdf\OpenWithProgids" /v CraftPdf.Portable /f >nul 2>&1
reg delete "HKCU\Software\RegisteredApplications" /v CraftPdf /f >nul 2>&1
echo CraftPdf sudah dihapus dari daftar aplikasi PDF.
pause
