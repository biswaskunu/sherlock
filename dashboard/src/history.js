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
  tabLive.setAttribute('aria-pressed', String(live));
  tabHistory.setAttribute('aria-pressed', String(!live));
  viewLive.classList.toggle('hidden', !live);
  viewHistory.classList.toggle('hidden', live);
}

const fromEl = document.getElementById('hist-from');
const toEl = document.getElementById('hist-to');
const fetchBtn = document.getElementById('hist-fetch');
const peakBtn = document.getElementById('hist-peak');
const histMeta = document.getElementById('hist-meta');
const corrTitle = document.getElementById('corr-title');
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
let histValues = [];
let selectedIndex = -1;
let fetchSeq = 0;

fetchBtn.addEventListener('click', fetchHistory);
peakBtn.addEventListener('click', () => {
  if (!histTimestamps.length) {
    histMeta.textContent = 'Scan a range first: there is no peak to invert yet.';
    return;
  }
  let best = 0;
  for (let i = 1; i < histValues.length; i++) {
    if (histValues[i] > histValues[best]) best = i;
  }
  paintSelection(best);
  correlate(histTimestamps[best]);
});

function paintSelection(index) {
  selectedIndex = index;
  if (!histChart) return;
  histChart.data.datasets[0].pointBackgroundColor = histChart.data.datasets[0].data.map(
    (_, i) => (i === index ? '#ffffff' : 'rgba(255, 255, 255, 0.25)'),
  );
  histChart.data.datasets[0].pointRadius = histChart.data.datasets[0].data.map((_, i) =>
    i === index ? 5 : 2,
  );
  histChart.update('none');
}

async function fetchHistory() {
  const mySeq = ++fetchSeq;
  const from = Math.floor(new Date(fromEl.value).getTime() / 1000);
  const to = Math.floor(new Date(toEl.value).getTime() / 1000);
  if (!Number.isFinite(from) || !Number.isFinite(to)) {
    histMeta.textContent = 'Invalid range: set both ends of the sweep.';
    return;
  }
  if (from > to) {
    histMeta.textContent = 'Start must be at or before end.';
    return;
  }
  histMeta.textContent = 'Scanning…';
  fetchBtn.disabled = true;
  peakBtn.disabled = true;
  try {
    const res = await fetch(`${backend}/api/metrics/history?from=${from}&to=${to}&limit=500`);
    if (mySeq !== fetchSeq) return; // stale response from an older click
    if (!res.ok) {
      const body = await res.text();
      histMeta.textContent = `Scan failed ${res.status}: ${body}: is the backend up?`;
      histTimestamps = [];
      histValues = [];
      return;
    }
    const { samples } = await res.json();
    const rows = Array.isArray(samples) ? samples : [];
    if (rows.length === 0) {
      histTimestamps = [];
      histValues = [];
      selectedIndex = -1;
      if (histChart) { histChart.destroy(); histChart = null; }
      histMeta.textContent =
        'Field absent in this range: the agent was not storing then.';
      corrTitle.textContent = 'No spike selected';
      corrMeta.textContent =
        'First light needs data: 01 run the agent · 02 spike the CPU · 03 scan this range · 04 click a column.';
      return;
    }
    histTimestamps = rows.map((s) => s.timestamp);
    histValues = rows.map((s) => s.global_cpu_pct);
    selectedIndex = -1;
    document.getElementById('hist').classList.remove('hidden');
    const labels = rows.map((s) => new Date(s.timestamp * 1000).toLocaleTimeString());
    const peak = Math.max(...histValues);
    histMeta.textContent = `${rows.length} frames · peak ${peak.toFixed(1)}%: click a column to invert it.`;

    if (histChart) histChart.destroy();
    histChart = new Chart(document.getElementById('hist'), {
      type: 'line',
      data: {
        labels,
        datasets: [
          {
            label: 'global_cpu_pct',
            data: histValues,
            borderColor: '#ffffff',
            backgroundColor: 'rgba(255, 255, 255, 0.06)',
            fill: true,
            borderWidth: 1.5,
            pointRadius: 2,
            pointBackgroundColor: 'rgba(255, 255, 255, 0.25)',
            pointBorderColor: '#ffffff',
            pointBorderWidth: 1,
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
            grid: { color: 'rgba(255, 255, 255, 0.14)' },
            ticks: { maxTicksLimit: 4 },
          },
        },
        onClick: (_ev, els) => {
          if (els.length > 0) {
            paintSelection(els[0].index);
            correlate(histTimestamps[els[0].index]);
          }
        },
      },
    });
  } catch (e) {
    if (mySeq !== fetchSeq) return;
    histTimestamps = [];
    histValues = [];
    histMeta.textContent = `Scan failed: ${e.message}: is the backend running?`;
  } finally {
    if (mySeq === fetchSeq) {
      fetchBtn.disabled = false;
      peakBtn.disabled = false;
    }
  }
}

async function correlate(timestamp) {
  if (!Number.isFinite(timestamp)) return;
  corrTitle.textContent = 'Spike forming…';
  corrMeta.textContent = `reading ${new Date(timestamp * 1000).toLocaleString()}…`;
  try {
    const res = await fetch(`${backend}/api/correlate?timestamp=${timestamp}&top_n=10`);
    if (res.status === 404) {
      corrTitle.textContent = 'Spike absent';
      corrMeta.textContent = 'No frame near that column (±5s): pick a denser part of the trace.';
      corrBody.innerHTML = '<tr><td colspan="4">field absent</td></tr>';
      return;
    }
    if (!res.ok) {
      corrTitle.textContent = 'Correlation failed';
      corrMeta.textContent = `Backend error ${res.status}: retry the column.`;
      return;
    }
    const { sample, top_processes } = await res.json();
    if (!sample || !Number.isFinite(sample.timestamp)) {
      corrTitle.textContent = 'Correlation failed';
      corrMeta.textContent = 'Unreadable correlate response: retry the column.';
      return;
    }
    const exact = sample.timestamp === timestamp;
    corrTitle.textContent = `Spike vivid: CPU ${sample.global_cpu_pct.toFixed(1)}%`;
    corrMeta.textContent =
      `${new Date(sample.timestamp * 1000).toLocaleString()} · ` +
      (exact ? 'exact frame' : 'nearest frame ±5s: treat ranks as approximate');
    const rows = top_processes ?? [];
    const peak = Math.max(0.1, ...rows.map((p) => p.cpu_pct));
    corrBody.innerHTML =
      rows
        .map(
          (p) =>
            `<tr><td>${p.pid}</td><td>${escapeHtml(p.name)}</td>` +
            `<td class="num"><span class="bar" aria-hidden="true"><i style="width:${Math.min(100, (p.cpu_pct / peak) * 100).toFixed(0)}%"></i></span>${p.cpu_pct.toFixed(1)}</td>` +
            `<td class="num">${p.mem_kb}</td></tr>`,
        )
        .join('') || '<tr><td colspan="4">no processes in this frame</td></tr>';
  } catch (e) {
    corrTitle.textContent = 'Correlation failed';
    corrMeta.textContent = `Request failed: ${e.message}: is the backend running?`;
  }
}

function escapeHtml(s) {
  return String(s).replace(
    /[&<>"']/g,
    (c) =>
      ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]),
  );
}
