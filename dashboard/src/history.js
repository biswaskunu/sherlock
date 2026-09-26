import { Chart } from 'chart.js';

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

fetchBtn.addEventListener('click', fetchHistory);

async function fetchHistory() {
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
  try {
    const res = await fetch(`${backend}/api/metrics/history?from=${from}&to=${to}&limit=500`);
    if (!res.ok) {
      const body = await res.text();
      histMeta.textContent = `error ${res.status}: ${body}`;
      return;
    }
    const { samples } = await res.json();
    histTimestamps = samples.map((s) => s.timestamp);
    const labels = samples.map((s) => new Date(s.timestamp * 1000).toLocaleTimeString());
    const data = samples.map((s) => s.global_cpu_pct);
    histMeta.textContent = `${samples.length} samples — click a point to correlate`;

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
    histMeta.textContent = `fetch failed: ${e.message} (is the backend running?)`;
  }
}

async function correlate(timestamp) {
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
