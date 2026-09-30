// The map page. Leaflet draws the pins; protomaps-leaflet draws the basemap from one
// PMTiles file on our own origin, so no third party sees who is looking at what.
(function () {
  var el = document.getElementById("map");
  var prefix = el.dataset.prefix;
  var dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  el.textContent = "";

  var map = L.map(el, { minZoom: 8, maxZoom: 16, maxBounds: [[8, 74.5], [13, 78]] }).setView([10.02, 76.5], 10);
  map.attributionControl.setPrefix(false);
  protomapsL
    .leafletLayer({
      url: "/tiles/ernakulam.pmtiles",
      flavor: dark ? "dark" : "light",
      lang: el.dataset.lang,
      maxDataZoom: 13,
      attribution: "© OpenStreetMap · Protomaps",
    })
    .addTo(map);

  var rupees = new Intl.NumberFormat("en-IN", { style: "currency", currency: "INR", maximumFractionDigits: 0 });

  function popup(p) {
    var box = document.createElement("div");
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
      // Flagged pins are drawn last so they sit on top.
      data.features
        .sort(function (a, b) { return a.properties.flagged - b.properties.flagged; })
        .forEach(function (f) {
          var c = f.geometry.coordinates;
          L.circleMarker([c[1], c[0]], {
            radius: f.properties.flagged ? 8 : 6,
            weight: 1.5,
            color: dark ? "#000" : "#fff",
            fillColor: f.properties.flagged ? "#ff7f41" : dark ? "#fff" : "#000",
            fillOpacity: 1,
          })
            .bindPopup(function () { return popup(f.properties); })
            .addTo(map);
        });
    });
})();
