import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface SystemInfo {
  version: string;
  apiVersion: string;
  runtime: string;
  os: string;
  arch: string;
  containersRunning: number;
  containersPaused: number;
  containersStopped: number;
  images: number;
  daemonConnected: boolean;
}

interface SystemState {
  daemonConnected: boolean;
  systemInfo: SystemInfo | null;
  theme: "light" | "dark" | "system";
  loading: boolean;
  error: string | null;
  checkDaemonStatus: () => Promise<void>;
  fetchSystemInfo: () => Promise<void>;
  startDaemon: () => Promise<void>;
  stopDaemon: () => Promise<void>;
  setTheme: (theme: "light" | "dark" | "system") => void;
}

export const useSystemStore = create<SystemState>((set, get) => ({
  daemonConnected: false,
  systemInfo: null,
  theme: "system",
  loading: false,
  error: null,

  checkDaemonStatus: async () => {
    try {
      const connected = await invoke<boolean>("check_daemon_status");
      set({ daemonConnected: connected });
      if (connected) {
        get().fetchSystemInfo();
      }
    } catch (error) {
      set({ daemonConnected: false });
    }
  },

  fetchSystemInfo: async () => {
    set({ loading: true, error: null });
    try {
      const info = await invoke<{
        version: string;
        api_version: string;
        runtime: string;
        os: string;
        arch: string;
        containers_running: number;
        containers_paused: number;
        containers_stopped: number;
        images: number;
        daemon_connected: boolean;
      }>("get_system_info");

      set({
        systemInfo: {
          version: info.version,
          apiVersion: info.api_version,
          runtime: info.runtime,
          os: info.os,
          arch: info.arch,
          containersRunning: info.containers_running,
          containersPaused: info.containers_paused,
          containersStopped: info.containers_stopped,
          images: info.images,
          daemonConnected: info.daemon_connected,
        },
        daemonConnected: info.daemon_connected,
        loading: false,
      });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  startDaemon: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("start_daemon");
      set({ daemonConnected: true, loading: false });
      get().fetchSystemInfo();
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  stopDaemon: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("stop_daemon");
      set({ daemonConnected: false, systemInfo: null, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  setTheme: (theme) => {
    set({ theme });
    if (
      theme === "dark" ||
      (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches)
    ) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  },
}));
