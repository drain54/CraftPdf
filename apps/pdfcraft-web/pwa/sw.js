// PeDeEp service worker: makes the web build installable and usable offline, and receives PDFs
// shared from other apps (Android share sheet -> PeDeEp).
//
// Strategy:
// - Page (navigations, index.html): network first, cached copy when offline.
// - Everything else from this origin (hashed .wasm/.js, icons, OCR models): cache first. Hashed
//   names change on every build, so a cached file is never stale.
// - A new deploy changes BUILD below (the workflow stamps the commit), which drops the old cache.
const BUILD = "__PEDEEP_BUILD__";
const CACHE = `pedeep-${BUILD}`;
const SHARED = "pedeep-shared";

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(["./", "./manifest.webmanifest", "./icon-192.png"])).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE && k !== SHARED).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

async function receiveShare(request) {
  try {
    const form = await request.formData();
    const file = form.getAll("pdf").find((f) => f && typeof f === "object");
    if (!file) return Response.redirect("./", 303);
    const name = (file.name || "shared.pdf").replace(/[\\/?#%]/g, "_");
    const path = `shared/${Date.now()}/${name}`;
    const cache = await caches.open(SHARED);
    // Keep only the latest few shared files.
    const old = await cache.keys();
    await Promise.all(old.slice(0, Math.max(0, old.length - 4)).map((k) => cache.delete(k)));
    await cache.put(new URL(path, self.registration.scope).href, new Response(file, { headers: { "Content-Type": "application/pdf" } }));
    return Response.redirect(`./?file=${encodeURIComponent(path)}`, 303);
  } catch (e) {
    return Response.redirect("./", 303);
  }
}

self.addEventListener("fetch", (event) => {
  const req = event.request;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;

  if (req.method === "POST" && url.pathname.endsWith("/share")) {
    event.respondWith(receiveShare(req));
    return;
  }
  if (req.method !== "GET") return;

  if (url.pathname.includes("/shared/")) {
    event.respondWith(caches.open(SHARED).then((c) => c.match(url.href)).then((r) => r || new Response("Shared file expired", { status: 404 })));
    return;
  }

  if (req.mode === "navigate") {
    event.respondWith(
      fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put("./", copy));
          }
          return res;
        })
        .catch(() => caches.open(CACHE).then((c) => c.match("./")).then((r) => r || Response.error())),
    );
    return;
  }

  event.respondWith(
    caches.open(CACHE).then((c) =>
      c.match(req).then(
        (hit) =>
          hit ||
          fetch(req).then((res) => {
            if (res.ok && res.type === "basic") c.put(req, res.clone());
            return res;
          }),
      ),
    ),
  );
});
