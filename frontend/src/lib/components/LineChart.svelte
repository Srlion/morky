<script>
  import { onDestroy, onMount } from "svelte";
  import {
    CategoryScale,
    Chart,
    Filler,
    Legend,
    LinearScale,
    LineController,
    LineElement,
    PointElement,
    Tooltip,
  } from "chart.js";

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    CategoryScale,
    Filler,
    Tooltip,
    Legend,
  );

  let {
    labels = [],
    datasets = [],
    yMax = 100,
    yUnit = "%",
    height = "200px",
  } = $props();

  let canvas;
  let chart;

  onMount(() => {
    chart = new Chart(canvas, {
      type: "line",
      data: { labels, datasets: buildDatasets(datasets) },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: { duration: 300 },
        interaction: {
          mode: "index",
          intersect: false,
        },
        plugins: {
          legend: {
            display: datasets.length > 1,
            labels: {
              color: "rgba(255,255,255,0.5)",
              boxWidth: 12,
              padding: 16,
              font: { size: 11 },
            },
          },
          tooltip: {
            backgroundColor: "rgba(0,0,0,0.8)",
            titleFont: {
              size: 11,
              family: "ui-monospace, monospace",
            },
            bodyFont: {
              size: 12,
              family: "ui-monospace, monospace",
            },
            padding: 10,
            cornerRadius: 8,
            callbacks: {
              label: (ctx) => `${ctx.dataset.label}: ${ctx.parsed.y}${yUnit}`,
            },
          },
        },
        scales: {
          x: {
            ticks: {
              color: "rgba(255,255,255,0.25)",
              font: {
                size: 10,
                family: "ui-monospace, monospace",
              },
              maxTicksLimit: 8,
              maxRotation: 0,
            },
            grid: { color: "rgba(255,255,255,0.04)" },
          },
          y: {
            min: 0,
            max: yMax,
            ticks: {
              color: "rgba(255,255,255,0.25)",
              font: {
                size: 10,
                family: "ui-monospace, monospace",
              },
              callback: (v) => `${v}${yUnit}`,
              stepSize: yMax / 4,
            },
            grid: {
              color: "rgba(255,255,255,0.06)",
            },
          },
        },
      },
    });
  });

  $effect(() => {
    if (!chart) return;
    chart.data.labels = labels;
    chart.data.datasets = buildDatasets(datasets);
    chart.update("none");
  });

  onDestroy(() => chart?.destroy());

  function buildDatasets(defs) {
    return defs.map((d) => ({
      label: d.label,
      data: d.data,
      borderColor: d.color,
      backgroundColor: d.color + "18",
      fill: true,
      tension: 0.3,
      borderWidth: 2,
      pointRadius: 0,
      pointHitRadius: 8,
      pointHoverRadius: 4,
      pointHoverBackgroundColor: d.color,
      ...d.extra,
    }));
  }
</script>

<div style="height: {height}; position: relative">
  <canvas bind:this={canvas}></canvas>
</div>
