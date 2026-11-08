// Search over search.json and the mobile menu. No external requests.
(function () {
  var q = document.getElementById('q'), box = document.getElementById('results'), data = null;
  var menu = document.querySelector('.menu'), find = document.querySelector('.find'), top = document.querySelector('.top');
  function setNav(open) {
    document.body.classList.toggle('nav-open', open);
    if (menu) menu.setAttribute('aria-expanded', open);
    if (open) { var cur = document.querySelector('.side a[aria-current]'); if (cur) cur.scrollIntoView({ block: 'center' }); }
  }
  function layout() {
    document.documentElement.style.setProperty('--top-h', (top.getBoundingClientRect().height + 8) + 'px');
    if (box) box.style.top = top.getBoundingClientRect().bottom + (document.body.classList.contains('search-open') ? 56 : 6) + 'px';
  }
  if (menu) menu.addEventListener('click', function () { setNav(!document.body.classList.contains('nav-open')); });
  var closeBtn = document.querySelector('.side .close');
  if (closeBtn) closeBtn.addEventListener('click', function () { setNav(false); });
  document.querySelectorAll('.side a').forEach(function (a) { a.addEventListener('click', function () { setNav(false); }); });
  if (find) find.addEventListener('click', function () {
    var on = document.body.classList.toggle('search-open');
    find.setAttribute('aria-expanded', on);
    layout();
    if (on) q.focus(); else box.hidden = true;
  });
  window.addEventListener('resize', function () { if (innerWidth > 800) setNav(false); layout(); });
  layout();
  function load(cb) {
    if (data) return cb();
    fetch('search.json').then(function (r) { return r.json(); }).then(function (d) { data = d; cb(); });
  }
  function esc(s) { return s.replace(/[&<>"]/g, function (c) { return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]; }); }
  function run() {
    var t = q.value.trim().toLowerCase();
    if (t.length < 2) { box.hidden = true; return; }
    var words = t.split(/\s+/), hits = [];
    data.forEach(function (p) {
      var hay = (p.title + ' ' + p.heads.map(function (h) { return h.t; }).join(' ') + ' ' + p.text).toLowerCase();
      if (!words.every(function (w) { return hay.indexOf(w) >= 0; })) return;
      var score = words.reduce(function (s, w) { return s + (p.title.toLowerCase().indexOf(w) >= 0 ? 10 : 0) +
        (p.heads.some(function (h) { return h.t.toLowerCase().indexOf(w) >= 0; }) ? 4 : 0); }, 0);
      var at = hay.indexOf(words[0], p.title.length), snip = at >= 0 ? p.text.substr(Math.max(0, at - p.title.length - 40), 140) : '';
      hits.push({ p: p, score: score, snip: snip });
    });
    hits.sort(function (a, b) { return b.score - a.score; });
    box.innerHTML = hits.length ? hits.slice(0, 12).map(function (h) {
      return '<a href="' + h.p.url + '"><strong>' + esc(h.p.title) + '</strong> <small>' + esc(h.p.group) + '</small><span>' + esc(h.snip) + '…</span></a>';
    }).join('') : '<p>No page matches “' + esc(q.value) + '”.</p>';
    box.hidden = false;
    layout();
  }
  q.addEventListener('input', function () { load(run); });
  q.addEventListener('keydown', function (e) { if (e.key === 'Escape') { box.hidden = true; q.blur(); setNav(false); } });
  document.addEventListener('keydown', function (e) {
    if (e.key === '/' && document.activeElement !== q && !/input|textarea/i.test(document.activeElement.tagName)) { e.preventDefault(); q.focus(); }
  });
  document.addEventListener('click', function (e) { if (!box.contains(e.target) && e.target !== q) box.hidden = true; });
})();
