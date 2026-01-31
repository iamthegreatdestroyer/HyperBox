import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  path: string;
  projectType: string;
  status: string;
  containers: string[];
  ports: number[];
  created: string;
}

export interface ProjectStatus {
  id: string;
  status: string;
  containersRunning: number;
  containersStopped: number;
  portsInUse: number[];
  resourceUsage: {
    cpuPercent: number;
    memoryMb: number;
    diskMb: number;
  };
}

interface ProjectState {
  projects: Project[];
  selectedProject: Project | null;
  projectStatus: Record<string, ProjectStatus>;
  loading: boolean;
  error: string | null;
  fetchProjects: () => Promise<void>;
  openProject: (path: string, name?: string) => Promise<void>;
  closeProject: (id: string, stopContainers?: boolean) => Promise<void>;
  startProject: (id: string) => Promise<void>;
  stopProject: (id: string) => Promise<void>;
  getProjectStatus: (id: string) => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  selectedProject: null,
  projectStatus: {},
  loading: false,
  error: null,

  fetchProjects: async () => {
    set({ loading: true, error: null });
    try {
      const rawProjects = await invoke<
        {
          id: string;
          name: string;
          path: string;
          project_type: string;
          status: string;
          containers: string[];
          ports: number[];
          created: string;
        }[]
      >("list_projects");

      const projects: Project[] = rawProjects.map((p) => ({
        id: p.id,
        name: p.name,
        path: p.path,
        projectType: p.project_type,
        status: p.status,
        containers: p.containers,
        ports: p.ports,
        created: p.created,
      }));

      set({ projects, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  openProject: async (path: string, name?: string) => {
    set({ loading: true, error: null });
    try {
      await invoke("open_project", { path, name });
      get().fetchProjects();
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  closeProject: async (id: string, stopContainers = true) => {
    try {
      await invoke("close_project", { id, stopContainers });
      get().fetchProjects();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  startProject: async (id: string) => {
    try {
      await invoke("start_project", { id });
      get().fetchProjects();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  stopProject: async (id: string) => {
    try {
      await invoke("stop_project", { id });
      get().fetchProjects();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  getProjectStatus: async (id: string) => {
    try {
      const rawStatus = await invoke<{
        id: string;
        status: string;
        containers_running: number;
        containers_stopped: number;
        ports_in_use: number[];
        resource_usage: {
          cpu_percent: number;
          memory_mb: number;
          disk_mb: number;
        };
      }>("get_project_status", { id });

      const status: ProjectStatus = {
        id: rawStatus.id,
        status: rawStatus.status,
        containersRunning: rawStatus.containers_running,
        containersStopped: rawStatus.containers_stopped,
        portsInUse: rawStatus.ports_in_use,
        resourceUsage: {
          cpuPercent: rawStatus.resource_usage.cpu_percent,
          memoryMb: rawStatus.resource_usage.memory_mb,
          diskMb: rawStatus.resource_usage.disk_mb,
        },
      };

      set((state) => ({
        projectStatus: { ...state.projectStatus, [id]: status },
      }));
    } catch (error) {
      console.error("Failed to fetch project status:", error);
    }
  },
}));
