@echo off
setlocal
rem Membatalkan "Jadikan PeDeEp default PDF.bat". Pilih aplikasi PDF lain dulu di Settings,
rem supaya Windows tidak menunjuk ke entri yang sudah dihapus.
echo Pilih dulu aplikasi PDF lain di Settings (klik ".pdf"), lalu kembali ke sini.
start "" "ms-settings:defaultapps"
pause
reg delete "HKCU\Software\Classes\PeDeEp.Portable" /f >nul 2>&1
reg delete "HKCU\Software\PeDeEp.Portable" /f >nul 2>&1
reg delete "HKCU\Software\Classes\.pdf\OpenWithProgids" /v PeDeEp.Portable /f >nul 2>&1
reg delete "HKCU\Software\RegisteredApplications" /v PeDeEp /f >nul 2>&1
echo PeDeEp sudah dihapus dari daftar aplikasi PDF.
pause
