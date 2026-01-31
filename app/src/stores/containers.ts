import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Container {
  id: string;
  name: string;
  image: string;
  status: string;
  created: string;
  ports: string[];
  projectId?: string;
  hasCheckpoint: boolean;
}

export interface ContainerStats {
  cpuPercent: number;
  memoryUsage: number;
  memoryLimit: number;
  memoryPercent: number;
  networkRx: number;
  networkTx: number;
  blockRead: number;
  blockWrite: number;
}

interface ContainerState {
  containers: Container[];
  selectedContainer: Container | null;
  stats: Record<string, ContainerStats>;
  loading: boolean;
  error: string | null;
  fetchContainers: (all?: boolean, projectId?: string) => Promise<void>;
  getContainer: (id: string) => Promise<void>;
  startContainer: (id: string) => Promise<void>;
  stopContainer: (id: string, createCheckpoint?: boolean) => Promise<void>;
  restartContainer: (id: string) => Promise<void>;
  removeContainer: (id: string, force?: boolean) => Promise<void>;
  fetchStats: (id: string) => Promise<void>;
}

export const useContainerStore = create<ContainerState>((set, get) => ({
  containers: [],
  selectedContainer: null,
  stats: {},
  loading: false,
  error: null,

  fetchContainers: async (all = true, projectId?: string) => {
    set({ loading: true, error: null });
    try {
      const containers = await invoke<Container[]>("list_containers", { all, projectId });
      set({ containers, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  getContainer: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const container = await invoke<Container>("get_container", { id });
      set({ selectedContainer: container, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  startContainer: async (id: string) => {
    try {
      await invoke("start_container", { id });
      get().fetchContainers();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  stopContainer: async (id: string, createCheckpoint = true) => {
    try {
      await invoke("stop_container", { id, createCheckpoint });
      get().fetchContainers();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  restartContainer: async (id: string) => {
    try {
      await invoke("restart_container", { id });
      get().fetchContainers();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  removeContainer: async (id: string, force = false) => {
    try {
      await invoke("remove_container", { id, force });
      get().fetchContainers();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  fetchStats: async (id: string) => {
    try {
      const rawStats = await invoke<{
        cpu_percent: number;
        memory_usage: number;
        memory_limit: number;
        memory_percent: number;
        network_rx: number;
        network_tx: number;
        block_read: number;
        block_write: number;
      }>("get_container_stats", { id });

      const stats: ContainerStats = {
        cpuPercent: rawStats.cpu_percent,
        memoryUsage: rawStats.memory_usage,
        memoryLimit: rawStats.memory_limit,
        memoryPercent: rawStats.memory_percent,
        networkRx: rawStats.network_rx,
        networkTx: rawStats.network_tx,
        blockRead: rawStats.block_read,
        blockWrite: rawStats.block_write,
      };

      set((state) => ({
        stats: { ...state.stats, [id]: stats },
      }));
    } catch (error) {
      console.error("Failed to fetch stats:", error);
    }
  },
}));
