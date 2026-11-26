/*!
 * Web Analytics Dev tracker v0.1.0 by Don PELY (MIT License)
 * Cookieless: no cookies, no IP storage, no cross-day identifiers.
 * Usage: <script defer src="http://127.0.0.1:3415/t.js" data-site="example.com"></script>
 * Custom events: wadev("Signup", { path: "/pricing" })
 */
(function () {
  'use strict';
  var el = document.currentScript;
  var site = (el && el.getAttribute('data-site')) || '';
  if (!site) return;
  var endpoint = new URL('/e', el.src).toString();
  site = site.toLowerCase();

  // Daily-rotating anonymous visitor id (stored in-memory + sessionStorage only).
  var day = new Date().toISOString().slice(0, 10);
  var visitor = '';
  try {
    var k = 'wadev_v_' + day;
    visitor = sessionStorage.getItem(k) || '';
    if (!visitor) {
      visitor = 'v' + Math.random().toString(36).slice(2) + Date.now().toString(36);
      sessionStorage.setItem(k, visitor);
    }
  } catch (e) {
    visitor = 'v' + Math.random().toString(36).slice(2);
  }

  function device() {
    var w = Math.min(window.screen.width, window.innerWidth || 9999);
    if (/Mobi|Android|iPhone|iPad|iPod/i.test(navigator.userAgent)) {
      return w >= 768 ? 'tablet' : 'mobile';
    }
    return 'desktop';
  }

  function browser() {
    var ua = navigator.userAgent;
    if (/Edg\//.test(ua)) return 'Edge';
    if (/OPR\//.test(ua)) return 'Opera';
    if (/Chrome\//.test(ua)) return 'Chrome';
    if (/Firefox\//.test(ua)) return 'Firefox';
    if (/Safari\//.test(ua)) return 'Safari';
    return 'Other';
  }

  function os() {
    var ua = navigator.userAgent;
    if (/Windows/.test(ua)) return 'Windows';
    if (/Mac OS/.test(ua)) return 'macOS';
    if (/Android/.test(ua)) return 'Android';
    if (/iPhone|iPad|iPod/.test(ua)) return 'iOS';
    if (/Linux/.test(ua)) return 'Linux';
    return 'Other';
  }

  function utm(name) {
    try {
      return new URLSearchParams(location.search).get(name) || '';
    } catch (e) {
      return '';
    }
  }

  function sourceFromReferrer(ref) {
    if (!ref) return '';
    try {
      var host = new URL(ref).hostname.replace(/^www\./, '');
      if (host === location.hostname.replace(/^www\./, '')) return '';
      return host;
    } catch (e) {
      return '';
    }
  }

  var queue = [];
  var flushing = false;
  function send(payload) {
    payload.site = site;
    payload.visitor = visitor;
    payload.session = visitor + ':' + day;
    payload.lang = (navigator.language || '').slice(0, 12);
    payload.device = device();
    payload.browser = browser();
    payload.os = os();
    var body = JSON.stringify(payload);
    if (navigator.sendBeacon) {
      try {
        if (navigator.sendBeacon(endpoint, body)) return;
      } catch (e) { /* fall through to fetch */ }
    }
    queue.push(body);
    flush();
  }

  function flush() {
    if (flushing || !queue.length) return;
    flushing = true;
    var body = queue.shift();
    fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body,
      keepalive: true,
    }).catch(function () {
      if (queue.length < 20) queue.push(body);
    }).finally(function () {
      flushing = false;
      if (queue.length) setTimeout(flush, 1000);
    });
  }

  function pageview() {
    var ref = document.referrer || '';
    send({
      kind: 'pageview',
      path: location.pathname + location.search,
      referrer: ref.slice(0, 500),
      source: utm('utm_source') || sourceFromReferrer(ref),
      medium: utm('utm_medium') || (ref ? 'referral' : 'direct'),
      campaign: utm('utm_campaign') || '',
    });
  }

  // Public API for custom events: wadev("Signup") or wadev("Purchase", { path: "/x" })
  window.wadev = function (name, props) {
    props = props || {};
    send({
      kind: 'event',
      name: String(name).slice(0, 120),
      path: props.path || location.pathname,
      source: sourceFromReferrer(document.referrer || ''),
      medium: document.referrer ? 'referral' : 'direct',
      campaign: '',
    });
  };

  // Track SPA navigations too.
  var lastPath = location.pathname + location.search;
  setInterval(function () {
    var p = location.pathname + location.search;
    if (p !== lastPath) {
      lastPath = p;
      pageview();
    }
  }, 1000);

  // Class-based events: <button class="wadev-event-Signup">…</button>
  document.addEventListener('click', function (ev) {
    var t = ev.target && ev.target.closest ? ev.target.closest('[class*="wadev-event-"]') : null;
    if (!t) return;
    var m = /wadev-event-([\w-]+)/.exec(t.className);
    if (m) window.wadev(m[1]);
  });

  if (document.readyState === 'complete') pageview();
  else window.addEventListener('load', pageview);
})();
