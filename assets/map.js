// The map page. Leaflet draws the pins; protomaps-leaflet draws the basemap from one
// PMTiles file on our own origin, so no third party sees who is looking at what.
(function () {
  var el = document.getElementById("map");
  var prefix = el.dataset.prefix;
  el.textContent = "";

  var map = L.map(el, { minZoom: 8, maxZoom: 16, maxBounds: [[8, 74.5], [13, 78]] }).setView([10.02, 76.5], 10);
  map.attributionControl.setPrefix(false);

  // Without the basemap file the pins still show, on a plain ground.
  fetch("/tiles/ernakulam.pmtiles", { method: "HEAD" }).then(function (r) {
    if (!r.ok) return;
    protomapsL
      .leafletLayer({
        url: "/tiles/ernakulam.pmtiles",
        flavor: "light",
        lang: el.dataset.lang,
        maxDataZoom: 13,
        attribution: "© OpenStreetMap · Protomaps",
      })
      .addTo(map);
  });

  var rupees = new Intl.NumberFormat("en-IN", { style: "currency", currency: "INR", maximumFractionDigits: 0 });

  function popup(p) {
    var box = document.createElement("div");
    box.className = "pop";
    var link = document.createElement("a");
    link.href = prefix + "/p/" + encodeURIComponent(p.code);
    link.textContent = p.title;
    link.lang = "en";
    box.appendChild(link);
    [p.work, p.amount ? rupees.format(p.amount) : null, p.code].forEach(function (line) {
      if (!line) return;
      var row = document.createElement("div");
      row.textContent = line;
      box.appendChild(row);
    });
    return box;
  }

  fetch("/api/v1/projects.geojson")
    .then(function (r) { return r.json(); })
    .then(function (data) {
      var wanted = decodeURIComponent(location.hash.slice(1));
      var found = [];
      // Flagged pins are drawn last so they sit on top.
      data.features
        .sort(function (a, b) { return a.properties.flagged - b.properties.flagged; })
        .forEach(function (f) {
          var c = f.geometry.coordinates;
          var marker = L.circleMarker([c[1], c[0]], {
            radius: f.properties.flagged ? 8 : 6,
            weight: 1.5,
            color: f.properties.flagged ? "#0a0a0a" : "#fff",
            fillColor: f.properties.flagged ? "#ff6a51" : "#0a0a0a",
            fillOpacity: 1,
          })
            .bindPopup(function () { return popup(f.properties); })
            .addTo(map);
          if (wanted && f.properties.code === wanted) found.push(marker);
        });
      // A link such as /map#PWD016-05-01 opens on that project.
      if (found.length) {
        map.setView(found[0].getLatLng(), 14);
        found[0].openPopup();
      }
    });
})();
