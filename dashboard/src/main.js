import { Chart, registerables } from 'chart.js';
import './history.js';

Chart.register(...registerables);

const WINDOW = 60; // 60 ticks x 3s = 3 min rolling window
const backend = import.meta.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080';
const url = `${backend}/api/metrics/live`;

const dot = document.getElementById('dot');
const meta = document.getElementById('meta');
const tbody = document.getElementById('procs');

function makeChart(el, label) {
  return new Chart(el, {
    type: 'line',
    data: { labels: [], datasets: [{ label, data: [], borderWidth: 1.5, pointRadius: 0 }] },
    options: { animation: false, scales: { x: { display: false } } },
  });
}

const cpuChart = makeChart(document.getElementById('cpu'), 'global_cpu_pct');
const memChart = makeChart(document.getElementById('mem'), 'used_mem_mb');

let lastAt = null;

function push(chart, label, value) {
  chart.data.labels.push(label);
  chart.data.datasets[0].data.push(value);
  if (chart.data.labels.length > WINDOW) {
    chart.data.labels.shift();
    chart.data.datasets[0].data.shift();
  }
  chart.update('none');
}

function setStatus(ok, text) {
  dot.className = ok ? 'ok' : 'bad';
  meta.textContent = text;
}

const es = new EventSource(url);
es.onopen = () => setStatus(true, `connected to ${url}`);
es.onerror = () => setStatus(false, `disconnected — retrying ${url}…`);
es.onmessage = (ev) => {
  try {
    const s = JSON.parse(ev.data);
    const t = new Date(s.timestamp * 1000).toLocaleTimeString();
    push(cpuChart, t, s.global_cpu_pct);
    push(memChart, t, Math.round(s.used_mem_kb / 1024));
    lastAt = Date.now();
    setStatus(true, `connected — last tick ${t} (cpu ${s.global_cpu_pct.toFixed(1)}%)`);

    tbody.innerHTML = (s.processes ?? [])
      .map(
        (p) =>
          `<tr><td>${p.pid}</td><td>${escapeHtml(p.name)}</td><td>${p.cpu_pct.toFixed(1)}</td><td>${p.mem_kb}</td></tr>`,
      )
      .join('') || '<tr><td colspan="4">no processes</td></tr>';
  } catch (e) {
    console.warn('bad SSE payload', e);
  }
};

// Detect stalls (agent stopped) even while SSE stays open via keep-alive.
setInterval(() => {
  if (lastAt && Date.now() - lastAt > 10000) {
    setStatus(false, 'stalled — no tick for 10s+ (is the agent running?)');
  }
}, 2000);

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
