# PeDeEp web di VPS (Ubuntu Server + Docker + Caddy)

PeDeEp versi browser: aplikasinya jalan di browser pengguna (WebAssembly), server hanya
mengirim file statis. Tidak ada database, PDF tidak pernah di-upload ke server.

**Pengganti "Edit in Paint":** klik kanan halaman ▸ *Edit page in image editor* (atau tombol
kuas di bar atas, atau klik kanan gambar ▸ *Edit in image editor*). Halaman terbuka di editor
gambar (miniPaint) di atas PeDeEp. Setiap **Ctrl+S** / *Simpan ke PDF* langsung mengganti
halaman di PeDeEp (satu langkah Undo per simpan), editor tetap terbuka. *Simpan & tutup* untuk
selesai. Jangan lupa simpan PDF-nya di PeDeEp setelah itu.

## Pasang (sekali)

Asumsi: Docker + Caddy proxy sudah jalan di `~/apps/proxy` dengan jaringan Docker `web`.

```bash
mkdir -p ~/apps/pedeep && cd ~/apps/pedeep
for f in docker-compose.yml Caddyfile update.sh; do
  curl -fLO "https://raw.githubusercontent.com/drain54/CraftPdf/main/packaging/pedeep-web/$f"
done
chmod +x update.sh
./update.sh            # unduh build terbaru ke www/current
docker compose up -d   # container "pedeep" di jaringan web
```

Lalu di Caddyfile proxy (`~/apps/proxy/Caddyfile`), di dalam blok situs `:80 { ... }`, sebelum
handler penangkap-semua:

```caddy
redir /pdf /pdf/ 308
handle_path /pdf/* {
	reverse_proxy pedeep:80
}
```

Muat ulang proxy: `cd ~/apps/proxy && docker compose exec caddy caddy reload --config /etc/caddy/Caddyfile`
(sesuaikan nama service-nya). Buka `http://IP-VPS/pdf/`.

## Update

Setiap push ke `main`, GitHub Actions membangun ulang dan menerbitkan rilis `web-latest`.
Di VPS cukup:

```bash
~/apps/pedeep/update.sh
```

Tidak perlu restart container. Rollback: `ls www/releases`, lalu
`ln -sfn releases/<versi> www/current`. Otomatis tiap malam (opsional):
`(crontab -l; echo "17 2 * * * $HOME/apps/pedeep/update.sh >/dev/null 2>&1") | crontab -`

## Catatan

- Jalan lewat HTTP biasa (IP saja) tidak masalah. Dengan domain + HTTPS nanti, cukup ganti
  blok situs proxy ke nama domain; Caddy mengurus sertifikatnya.
- Beban server hampir nol: semua render, OCR, dan edit gambar dikerjakan di browser. Halaman
  scan besar butuh RAM di laptop pengguna, bukan di VPS.
- Ukuran unduhan pertama ±beberapa puluh MB (aplikasi + model OCR saat OCR dipakai), setelahnya
  di-cache browser.
- Editor gambar: [miniPaint](https://github.com/viliusle/miniPaint) (MIT), di-pin per commit di
  `.github/workflows/pedeep-web.yml`, disajikan dari `paint/` di situs yang sama.
