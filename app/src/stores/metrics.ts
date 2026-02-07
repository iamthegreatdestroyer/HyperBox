import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface MetricsSnapshot {
  timestamp: number;
  cpu: number; // total CPU % across all containers
  memory: number; // total memory bytes
  memoryLimit: number;
  networkRx: number; // bytes/s
  networkTx: number; // bytes/s
  blockRead: number; // bytes/s
  blockWrite: number; // bytes/s
}

export interface ContainerMetricsSnapshot {
  timestamp: number;
  containerId: string;
  containerName: string;
  cpu: number;
  memory: number;
  memoryLimit: number;
  memoryPercent: number;
  networkRx: number;
  networkTx: number;
  blockRead: number;
  blockWrite: number;
}

export interface MemoryPressure {
  level: "low" | "moderate" | "high" | "critical";
  totalUsed: number;
  totalLimit: number;
  percent: number;
  containersAtRisk: string[];
}

export interface ChartDataPoint {
  time: string;
  cpu: number;
  memory: number;
  networkRx: number;
  networkTx: number;
  blockRead: number;
  blockWrite: number;
}

export interface ContainerChartDataPoint {
  time: string;
  cpu: number;
  memory: number;
  memoryPercent: number;
  networkRx: number;
  networkTx: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_HISTORY_POINTS = 60; // 60 data points → 5 min at 5s interval
const METRICS_INTERVAL_MS = 3000; // 3-second refresh for streaming feel

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface MetricsState {
  // Aggregated system-level time-series
  systemHistory: ChartDataPoint[];

  // Per-container time-series (keyed by container id)
  containerHistories: Record<string, ContainerChartDataPoint[]>;

  // Latest per-container snapshot for cards
  latestContainerMetrics: Record<string, ContainerMetricsSnapshot>;

  // Memory pressure indicator
  memoryPressure: MemoryPressure;

  // Streaming state
  streaming: boolean;
  intervalId: number | null;

  // Actions
  startStreaming: () => void;
  stopStreaming: () => void;
  tick: () => Promise<void>;
  getContainerHistory: (id: string) => ContainerChartDataPoint[];
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

function computeMemoryPressure(
  containerMetrics: Record<string, ContainerMetricsSnapshot>,
): MemoryPressure {
  let totalUsed = 0;
  let totalLimit = 0;
  const containersAtRisk: string[] = [];

  for (const m of Object.values(containerMetrics)) {
    totalUsed += m.memory;
    totalLimit += m.memoryLimit;
    if (m.memoryPercent > 80) {
      containersAtRisk.push(m.containerName || m.containerId.slice(0, 12));
    }
  }

  const percent = totalLimit > 0 ? (totalUsed / totalLimit) * 100 : 0;

  let level: MemoryPressure["level"] = "low";
  if (percent > 90) level = "critical";
  else if (percent > 75) level = "high";
  else if (percent > 50) level = "moderate";

  return { level, totalUsed, totalLimit, percent, containersAtRisk };
}

function appendPoint<T>(arr: T[], point: T, max: number): T[] {
  const next = [...arr, point];
  return next.length > max ? next.slice(next.length - max) : next;
}

export const useMetricsStore = create<MetricsState>((set, get) => ({
  systemHistory: [],
  containerHistories: {},
  latestContainerMetrics: {},
  memoryPressure: {
    level: "low",
    totalUsed: 0,
    totalLimit: 0,
    percent: 0,
    containersAtRisk: [],
  },
  streaming: false,
  intervalId: null,

  startStreaming: () => {
    const { streaming } = get();
    if (streaming) return;

    // Initial tick
    get().tick();

    const id = window.setInterval(() => {
      get().tick();
    }, METRICS_INTERVAL_MS);

    set({ streaming: true, intervalId: id });
  },

  stopStreaming: () => {
    const { intervalId } = get();
    if (intervalId !== null) {
      window.clearInterval(intervalId);
    }
    set({ streaming: false, intervalId: null });
  },

  tick: async () => {
    try {
      // Fetch list of running containers
      const containers = await invoke<Array<{ id: string; name: string; status: string }>>(
        "list_containers",
        { all: false },
      );

      const runningIds = containers
        .filter((c) => c.status === "running")
        .map((c) => ({ id: c.id, name: c.name }));

      if (runningIds.length === 0) {
        const ts = Date.now();
        const time = formatTime(ts);
        const zeroPoint: ChartDataPoint = {
          time,
          cpu: 0,
          memory: 0,
          networkRx: 0,
          networkTx: 0,
          blockRead: 0,
          blockWrite: 0,
        };

        set((state) => ({
          systemHistory: appendPoint(state.systemHistory, zeroPoint, MAX_HISTORY_POINTS),
          memoryPressure: {
            level: "low",
            totalUsed: 0,
            totalLimit: 0,
            percent: 0,
            containersAtRisk: [],
          },
        }));
        return;
      }

      // Fetch stats for each running container
      const rawResults = await Promise.allSettled(
        runningIds.map(async ({ id, name }) => {
          const raw = await invoke<{
            cpu_percent: number;
            memory_usage: number;
            memory_limit: number;
            memory_percent: number;
            network_rx: number;
            network_tx: number;
            block_read: number;
            block_write: number;
          }>("get_container_stats", { id });

          return {
            containerId: id,
            containerName: name,
            cpu: raw.cpu_percent,
            memory: raw.memory_usage,
            memoryLimit: raw.memory_limit,
            memoryPercent: raw.memory_percent,
            networkRx: raw.network_rx,
            networkTx: raw.network_tx,
            blockRead: raw.block_read,
            blockWrite: raw.block_write,
          };
        }),
      );

      const ts = Date.now();
      const time = formatTime(ts);

      // Aggregate stats
      let totalCpu = 0;
      let totalMem = 0;
      let totalMemLimit = 0;
      let totalNetRx = 0;
      let totalNetTx = 0;
      let totalBlockR = 0;
      let totalBlockW = 0;

      const newLatest: Record<string, ContainerMetricsSnapshot> = {};
      const newContainerHistories = { ...get().containerHistories };

      for (const result of rawResults) {
        if (result.status !== "fulfilled") continue;
        const m = result.value;

        totalCpu += m.cpu;
        totalMem += m.memory;
        totalMemLimit += m.memoryLimit;
        totalNetRx += m.networkRx;
        totalNetTx += m.networkTx;
        totalBlockR += m.blockRead;
        totalBlockW += m.blockWrite;

        newLatest[m.containerId] = { timestamp: ts, ...m };

        const cPoint: ContainerChartDataPoint = {
          time,
          cpu: m.cpu,
          memory: m.memory,
          memoryPercent: m.memoryPercent,
          networkRx: m.networkRx,
          networkTx: m.networkTx,
        };

        newContainerHistories[m.containerId] = appendPoint(
          newContainerHistories[m.containerId] || [],
          cPoint,
          MAX_HISTORY_POINTS,
        );
      }

      const sysPoint: ChartDataPoint = {
        time,
        cpu: totalCpu,
        memory: totalMem,
        networkRx: totalNetRx,
        networkTx: totalNetTx,
        blockRead: totalBlockR,
        blockWrite: totalBlockW,
      };

      const pressure = computeMemoryPressure(newLatest);

      set((state) => ({
        systemHistory: appendPoint(state.systemHistory, sysPoint, MAX_HISTORY_POINTS),
        containerHistories: newContainerHistories,
        latestContainerMetrics: newLatest,
        memoryPressure: pressure,
      }));
    } catch (error) {
      // Silently handle — daemon might not be connected
      console.warn("Metrics tick failed:", error);
    }
  },

  getContainerHistory: (id: string) => {
    return get().containerHistories[id] || [];
  },
}));
