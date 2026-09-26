import { Chart, registerables } from 'chart.js';
import './style.css';
import './history.js';

Chart.register(...registerables);

Chart.defaults.color = '#ffffff';
Chart.defaults.borderColor = 'rgba(255, 255, 255, 0.14)';
Chart.defaults.font.family =
  'ui-monospace, "Cascadia Mono", Menlo, Consolas, monospace';
Chart.defaults.font.size = 10;

const WINDOW = 60; // 60 ticks x 3s = 3 min rolling window
const backend = import.meta.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080';
const url = `${backend}/api/metrics/live`;

const status = document.getElementById('status');
const meta = document.getElementById('meta');
const metaSub = document.getElementById('meta-sub');
const tbody = document.getElementById('procs');
const ledgerAddr = document.getElementById('ledger-addr');
const statCpu = document.getElementById('stat-cpu');
const statMem = document.getElementById('stat-mem');
const statTick = document.getElementById('stat-tick');
const strip = document.getElementById('strip');

function makeChart(el, label, max) {
  return new Chart(el, {
    type: 'line',
    data: {
      labels: [],
      datasets: [
        {
          label,
          data: [],
          borderColor: '#ffffff',
          backgroundColor: 'rgba(255, 255, 255, 0.06)',
          fill: true,
          borderWidth: 1.5,
          pointRadius: 0,
          stepped: false,
        },
      ],
    },
    options: {
      animation: false,
      plugins: { legend: { display: false } },
      scales: {
        x: { display: false },
        y: {
          min: 0,
          ...(max === undefined ? {} : { max }),
          grid: { color: 'rgba(255, 255, 255, 0.14)' },
          ticks: { maxTicksLimit: 4 },
        },
      },
    },
  });
}

const cpuChart = makeChart(document.getElementById('cpu'), 'global_cpu_pct', 100);
const memChart = makeChart(document.getElementById('mem'), 'used_mem_mb');

let lastAt = null;
const stripFrames = [];

function push(chart, label, value) {
  chart.data.labels.push(label);
  chart.data.datasets[0].data.push(value);
  if (chart.data.labels.length > WINDOW) {
    chart.data.labels.shift();
    chart.data.datasets[0].data.shift();
  }
  chart.update('none');
}

function setStatus(mode, text, sub) {
  status.classList.toggle('is-live', mode === 'live');
  status.classList.toggle('is-stalled', mode === 'stalled');
  status.classList.toggle('is-dead', mode === 'dead');
  meta.textContent = text;
  metaSub.textContent = sub;
}

function drawStrip() {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = strip.clientWidth;
  const h = 64;
  if (!w) return;
  strip.width = w * dpr;
  strip.height = h * dpr;
  const ctx = strip.getContext('2d');
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);
  const n = stripFrames.length;
  if (!n) return;
  const bw = Math.max(1, Math.floor(w / 120));
  const count = Math.min(n, Math.floor(w / (bw + 1)));
  for (let i = 0; i < count; i++) {
    const v = stripFrames[n - count + i];
    const bh = Math.max(2, Math.round((v / 100) * (h - 12)));
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(w - (count - i) * (bw + 1), h - 6 - bh, bw, bh);
  }
}

window.addEventListener('resize', drawStrip);

const es = new EventSource(url);
es.onopen = () => setStatus('live', 'LIVE', `sweeping ${url}`);
es.onerror = () =>
  setStatus('dead', 'OFFLINE: RETRYING', `no carrier on ${url}: is the backend up?`);
es.onmessage = (ev) => {
  try {
    const s = JSON.parse(ev.data);
    strip.classList.remove('hidden');
    const t = new Date(s.timestamp * 1000).toLocaleTimeString();
    push(cpuChart, t, s.global_cpu_pct);
    push(memChart, t, Math.round(s.used_mem_kb / 1024));
    stripFrames.push(s.global_cpu_pct);
    if (stripFrames.length > 240) stripFrames.shift();
    drawStrip();
    lastAt = Date.now();
    statCpu.textContent = s.global_cpu_pct.toFixed(1);
    statMem.textContent = String(Math.round(s.used_mem_kb / 1024));
    statTick.textContent = t;
    setStatus(
      'live',
      'LIVE',
      `frame ${t} · cpu ${s.global_cpu_pct.toFixed(1)}% · ${url}`,
    );
    ledgerAddr.textContent = `frame ${t}`;

    const rows = s.processes ?? [];
    const peak = Math.max(0.1, ...rows.map((p) => p.cpu_pct));
    tbody.innerHTML =
      rows
        .map(
          (p) =>
            `<tr><td>${p.pid}</td><td>${escapeHtml(p.name)}</td>` +
            `<td class="num"><span class="bar" aria-hidden="true"><i style="width:${Math.min(100, (p.cpu_pct / peak) * 100).toFixed(0)}%"></i></span>${p.cpu_pct.toFixed(1)}</td>` +
            `<td class="num">${p.mem_kb}</td></tr>`,
        )
        .join('') || '<tr><td colspan="4">no processes in this frame</td></tr>';
  } catch (e) {
    console.warn('bad SSE payload', e);
  }
};

// Detect stalls (agent stopped) even while SSE stays open via keep-alive.
setInterval(() => {
  if (lastAt && Date.now() - lastAt > 10000) {
    setStatus(
      'stalled',
      'STALLED: NO TICK 10S+',
      'carrier open, frames absent: is the agent running?',
    );
  }
}, 2000);

function escapeHtml(s) {
  return String(s).replace(
    /[&<>"']/g,
    (c) =>
      ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]),
  );
}
