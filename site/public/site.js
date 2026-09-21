/* innards.app — the little bit of JS the site needs. No dependencies.
   Everything here is progressive enhancement: the pages read fine without it. */
(function () {
  "use strict";
  document.documentElement.classList.remove("no-js");

  /* ---------------------------------------------------------- level demo
     A tablist (plain / informed / expert) and an optional finding switch.
     Each finding is an <article data-finding> holding one .finding-body per
     level; we just hide the bodies that don't match the selected level. */
  var demo = document.querySelector("[data-level-demo]");
  if (demo) {
    var tabs = Array.prototype.slice.call(demo.querySelectorAll("[role=tab]"));
    var findings = Array.prototype.slice.call(demo.querySelectorAll("[data-finding]"));
    var switches = Array.prototype.slice.call(demo.querySelectorAll("[data-pick-finding]"));

    function setLevel(level) {
      tabs.forEach(function (t) {
        var on = t.getAttribute("data-level") === level;
        t.setAttribute("aria-selected", on ? "true" : "false");
        t.tabIndex = on ? 0 : -1;
      });
      demo.querySelectorAll(".finding-body").forEach(function (b) {
        b.hidden = b.getAttribute("data-level") !== level;
      });
    }
    function setFinding(id) {
      findings.forEach(function (f) { f.hidden = f.getAttribute("data-finding") !== id; });
      switches.forEach(function (s) {
        s.setAttribute("aria-pressed", s.getAttribute("data-pick-finding") === id ? "true" : "false");
      });
    }

    tabs.forEach(function (tab, i) {
      tab.addEventListener("click", function () { setLevel(tab.getAttribute("data-level")); });
      tab.addEventListener("keydown", function (e) {
        var next = null;
        if (e.key === "ArrowRight" || e.key === "ArrowDown") next = tabs[(i + 1) % tabs.length];
        if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = tabs[(i - 1 + tabs.length) % tabs.length];
        if (e.key === "Home") next = tabs[0];
        if (e.key === "End") next = tabs[tabs.length - 1];
        if (next) { e.preventDefault(); next.focus(); setLevel(next.getAttribute("data-level")); }
      });
    });
    switches.forEach(function (s) {
      s.addEventListener("click", function () { setFinding(s.getAttribute("data-pick-finding")); });
    });

    var initial = tabs.filter(function (t) { return t.getAttribute("aria-selected") === "true"; })[0] || tabs[0];
    if (initial) setLevel(initial.getAttribute("data-level"));
    if (findings.length) setFinding(findings[0].getAttribute("data-finding"));
  }

  /* ---------------------------------------------------------- download page
     Guess the visitor's OS and lift that card to the front. Nothing is sent
     anywhere; this only reads navigator.platform / userAgent locally. */
  var cards = document.querySelectorAll("[data-os]");
  if (cards.length) {
    var ua = (navigator.userAgent || "").toLowerCase();
    var plat = (navigator.platform || "").toLowerCase();
    var uad = navigator.userAgentData && navigator.userAgentData.platform;
    var os = null;
    if (uad) uad = String(uad).toLowerCase();
    if ((uad && uad.indexOf("mac") === 0) || /mac/.test(plat) || /macintosh/.test(ua)) os = "macos";
    else if ((uad && uad.indexOf("win") === 0) || /win/.test(plat) || /windows/.test(ua)) os = "windows";
    else if ((uad && uad.indexOf("linux") === 0) || /linux|x11|bsd/.test(plat) || /linux/.test(ua)) os = "linux";
    if (/iphone|ipad|android/.test(ua)) os = null;
    if (os) {
      cards.forEach(function (c) {
        var mine = c.getAttribute("data-os") === os;
        c.classList.toggle("current", mine);
        var tag = c.querySelector("[data-detected]");
        if (tag) tag.hidden = !mine;
        if (mine && c.parentNode.firstElementChild !== c) c.parentNode.insertBefore(c, c.parentNode.firstElementChild);
      });
    }
  }

})();
