// Keeps pages you have opened readable without a connection.
// Pages: the network first, the saved copy when it fails. Fonts, icons and scripts: the saved
// copy first, refreshed in the background.
var PAGES = "pages-v1";
var STATIC = "static-v1";
var KEEP = 80; // saved pages; the oldest are dropped beyond this
var SHELL = ["/offline", "/en/offline", "/favicon.svg", "/manifest.webmanifest"];

self.addEventListener("install", function (event) {
  event.waitUntil(caches.open(STATIC).then(function (cache) { return cache.addAll(SHELL); }).then(function () { return self.skipWaiting(); }));
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches.keys().then(function (names) {
      return Promise.all(names.filter(function (n) { return n !== PAGES && n !== STATIC; }).map(function (n) { return caches.delete(n); }));
    }).then(function () { return self.clients.claim(); })
  );
});

function trim(cache) {
  return cache.keys().then(function (keys) {
    return keys.length > KEEP ? Promise.all(keys.slice(0, keys.length - KEEP).map(function (k) { return cache.delete(k); })) : null;
  });
}

// A copy of a page that remembers when it was saved.
function stamped(response) {
  var headers = new Headers(response.headers);
  headers.set("X-Saved-At", new Date().toISOString());
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers: headers });
}

// A saved page with a line at the top saying it is a saved copy and from which day.
function dated(saved, english) {
  var at = new Date(saved.headers.get("X-Saved-At") || saved.headers.get("Date") || "");
  // The day in India, written as the rest of the site writes dates. Copies saved before this was added have none.
  var day = isNaN(at) ? "" : new Date(at.getTime() + 330 * 60000).toISOString().slice(0, 10).split("-").reverse().join("-");
  var text = english
    ? "You are offline. This is " + (day ? "the copy saved on " + day : "a saved copy") + "; the figures may have changed since."
    : "ഇപ്പോൾ ഇന്റർനെറ്റ് ഇല്ല. ഇത് " + (day ? day + "-ന് സൂക്ഷിച്ച പകർപ്പാണ്" : "മുമ്പ് സൂക്ഷിച്ച പകർപ്പാണ്") + "; അതിനുശേഷം കണക്കുകൾ മാറിയിരിക്കാം.";
  var note = '<p class="stale" role="status">' + text + "</p>";
  var headers = new Headers(saved.headers);
  ["Content-Length", "Content-Encoding", "ETag"].forEach(function (name) { headers.delete(name); });
  return saved.text().then(function (html) {
    return new Response(html.replace('<main id="main">', note + '<main id="main">'), { status: saved.status, statusText: saved.statusText, headers: headers });
  });
}

self.addEventListener("fetch", function (event) {
  var request = event.request;
  var url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) return;
  // Data, stored copies, map tiles and share images are never kept: they are large or must be current.
  if (/^\/(api|snapshot|tiles|admin|og)\//.test(url.pathname)) return;

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request).then(function (response) {
        if (response.ok) {
          var copy = stamped(response.clone());
          caches.open(PAGES).then(function (cache) { return cache.put(request, copy).then(function () { return trim(cache); }); });
        }
        return response;
      }).catch(function () {
        var english = /^\/en(\/|$)/.test(url.pathname);
        return caches.match(request).then(function (saved) {
          return saved ? dated(saved, english) : caches.match(english ? "/en/offline" : "/offline");
        });
      })
    );
    return;
  }

  // Fonts, icons and scripts: the saved copy at once, refreshed in the background for next time.
  if (/^\/(fonts|vendor)\//.test(url.pathname) || /\.(png|svg|webmanifest|js)$/.test(url.pathname)) {
    event.respondWith(
      caches.open(STATIC).then(function (cache) {
        return cache.match(request).then(function (saved) {
          var fresh = fetch(request).then(function (response) {
            if (response.ok) cache.put(request, response.clone());
            return response;
          });
          if (!saved) return fresh;
          fresh.catch(function () {});
          return saved;
        });
      })
    );
  }
});
