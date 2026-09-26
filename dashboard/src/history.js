import { Chart, registerables } from 'chart.js';

Chart.register(...registerables);

const backend = import.meta.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080';

const tabLive = document.getElementById('tab-live');
const tabHistory = document.getElementById('tab-history');
const viewLive = document.getElementById('view-live');
const viewHistory = document.getElementById('view-history');

tabLive.addEventListener('click', () => switchTab(true));
tabHistory.addEventListener('click', () => switchTab(false));

function switchTab(live) {
  tabLive.classList.toggle('active', live);
  tabHistory.classList.toggle('active', !live);
  viewLive.classList.toggle('hidden', !live);
  viewHistory.classList.toggle('hidden', live);
}

const fromEl = document.getElementById('hist-from');
const toEl = document.getElementById('hist-to');
const fetchBtn = document.getElementById('hist-fetch');
const histMeta = document.getElementById('hist-meta');
const corrMeta = document.getElementById('corr-meta');
const corrBody = document.getElementById('corr-procs');

// Default range: last hour.
{
  const now = new Date();
  const hourAgo = new Date(now.getTime() - 60 * 60 * 1000);
  toEl.value = toLocalInput(now);
  fromEl.value = toLocalInput(hourAgo);
}

function toLocalInput(d) {
  const pad = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

let histChart = null;
let histTimestamps = [];
let fetchSeq = 0;

fetchBtn.addEventListener('click', fetchHistory);

async function fetchHistory() {
  const mySeq = ++fetchSeq;
  const from = Math.floor(new Date(fromEl.value).getTime() / 1000);
  const to = Math.floor(new Date(toEl.value).getTime() / 1000);
  if (!Number.isFinite(from) || !Number.isFinite(to)) {
    histMeta.textContent = 'invalid range — pick both dates';
    return;
  }
  if (from > to) {
    histMeta.textContent = 'from must be <= to';
    return;
  }
  histMeta.textContent = 'loading…';
  fetchBtn.disabled = true;
  try {
    const res = await fetch(`${backend}/api/metrics/history?from=${from}&to=${to}&limit=500`);
    if (mySeq !== fetchSeq) return; // stale response from an older click
    if (!res.ok) {
      const body = await res.text();
      histMeta.textContent = `error ${res.status}: ${body}`;
      histTimestamps = [];
      return;
    }
    const { samples } = await res.json();
    const rows = Array.isArray(samples) ? samples : [];
    if (rows.length === 0) {
      histTimestamps = [];
      if (histChart) { histChart.destroy(); histChart = null; }
      histMeta.textContent = 'no samples in range — agent may not have been running then';
      return;
    }
    histTimestamps = rows.map((s) => s.timestamp);
    const labels = rows.map((s) => new Date(s.timestamp * 1000).toLocaleTimeString());
    const data = rows.map((s) => s.global_cpu_pct);
    histMeta.textContent = `${rows.length} samples — click a point to correlate`;

    if (histChart) histChart.destroy();
    histChart = new Chart(document.getElementById('hist'), {
      type: 'line',
      data: { labels, datasets: [{ label: 'global_cpu_pct', data, borderWidth: 1.5, pointRadius: 2 }] },
      options: {
        animation: false,
        onClick: (_ev, els) => {
          if (els.length > 0) correlate(histTimestamps[els[0].index]);
        },
      },
    });
  } catch (e) {
    if (mySeq !== fetchSeq) return;
    histTimestamps = [];
    histMeta.textContent = `fetch failed: ${e.message} (is the backend running?)`;
  } finally {
    if (mySeq === fetchSeq) fetchBtn.disabled = false;
  }
}

async function correlate(timestamp) {
  if (!Number.isFinite(timestamp)) return;
  corrMeta.textContent = `correlating ${new Date(timestamp * 1000).toLocaleString()}…`;
  try {
    const res = await fetch(`${backend}/api/correlate?timestamp=${timestamp}&top_n=10`);
    if (res.status === 404) {
      corrMeta.textContent = 'no sample near that timestamp (±5s)';
      corrBody.innerHTML = '<tr><td colspan="4">no data</td></tr>';
      return;
    }
    if (!res.ok) {
      corrMeta.textContent = `error ${res.status}`;
      return;
    }
    const { sample, top_processes } = await res.json();
    if (!sample || !Number.isFinite(sample.timestamp)) {
      corrMeta.textContent = 'bad correlate response';
      return;
    }
    corrMeta.textContent = `spike at ${new Date(sample.timestamp * 1000).toLocaleString()} — cpu ${sample.global_cpu_pct.toFixed(1)}%`;
    corrBody.innerHTML =
      (top_processes ?? [])
        .map(
          (p) =>
            `<tr><td>${p.pid}</td><td>${escapeHtml(p.name)}</td><td>${p.cpu_pct.toFixed(1)}</td><td>${p.mem_kb}</td></tr>`,
        )
        .join('') || '<tr><td colspan="4">no processes</td></tr>';
  } catch (e) {
    corrMeta.textContent = `correlate failed: ${e.message}`;
  }
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
