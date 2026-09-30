// The only script on ordinary pages: it lets pages you have opened work offline.
if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js");
