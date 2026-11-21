#!/usr/bin/env bash
# Download the newest PeDeEp web build (install and update). No restart needed: Caddy serves
# the new files at once; users get them on their next page load.
#
# Layout: www/releases/<version>/ holds each build; www/current points at the live one, so the
# switch is a single rename and the previous build stays for a rollback (keeps the last 3).
set -euo pipefail
cd "$(dirname "$0")"
REPO="${PEDEEP_REPO:-drain54/CraftPdf}"
URL="${PEDEEP_URL:-https://github.com/$REPO/releases/download/web-latest/PeDeEp-web.tar.gz}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Downloading $URL"
curl -fL --retry 3 -o "$tmp/site.tar.gz" "$URL"
mkdir -p "$tmp/site"
tar xzf "$tmp/site.tar.gz" -C "$tmp/site"
[[ -f "$tmp/site/index.html" && -f "$tmp/site/paint/index.html" ]] || { echo "error: the download is not a PeDeEp web build" >&2; exit 1; }
version="$(tr -cd '0-9a-zA-Z' < "$tmp/site/VERSION" 2>/dev/null || true)"
version="${version:-$(date +%Y%m%d%H%M%S)}"
mkdir -p www/releases
if [[ "$(readlink www/current 2>/dev/null || true)" == "releases/$version" ]]; then
  echo "PeDeEp web $version is already live"
  exit 0
fi
rm -rf "www/releases/$version"
mv "$tmp/site" "www/releases/$version"
chmod -R a+rX "www/releases/$version"
ln -sfn "releases/$version" www/current.new
mv -T www/current.new www/current
# Keep the newest three builds.
ls -1t www/releases | tail -n +4 | while read -r old; do rm -rf "www/releases/$old"; done
echo "PeDeEp web $version is live"
