/**
 * Mock for @tauri-apps/api
 * Provides mock implementations for Tauri API calls in tests
 */

import { vi } from "vitest";

// Mock container data
export const mockContainers = [
  {
    id: "abc123def456",
    name: "web-server",
    image: "nginx:latest",
    status: "running",
    ports: ["8080:80/tcp"],
    projectId: "project-1",
    created: new Date().toISOString(),
    hasCheckpoint: false,
  },
  {
    id: "xyz789ghi012",
    name: "database",
    image: "postgres:15",
    status: "stopped",
    ports: ["5432:5432/tcp"],
    projectId: "project-1",
    created: new Date().toISOString(),
    hasCheckpoint: false,
  },
];

// Mock project data
export const mockProjects = [
  {
    id: "project-1",
    name: "my-web-app",
    path: "/home/user/projects/my-web-app",
    project_type: "docker-compose",
    status: "running",
    containers: ["abc123def456", "xyz789ghi012"],
    ports: [8080, 5432],
    created: new Date().toISOString(),
  },
];

// Mock image data
export const mockImages = [
  {
    id: "sha256:abc123",
    tags: ["nginx:latest"],
    size: 142000000,
    created: new Date().toISOString(),
    in_use: true,
  },
  {
    id: "sha256:def456",
    tags: ["postgres:15"],
    size: 385000000,
    created: new Date().toISOString(),
    in_use: false,
  },
];

// Mock system info - snake_case to match backend API
export const mockSystemInfo = {
  version: "0.1.0",
  api_version: "1.0",
  runtime: "hyperbox",
  os: "windows",
  arch: "x86_64",
  containers_running: 1,
  containers_paused: 0,
  containers_stopped: 1,
  images: 2,
  daemon_connected: true, // Critical: enables Dashboard fetches
};

// Mock performance metrics - snake_case to match backend API
export const mockPerformanceMetrics = {
  cold_start_avg_ms: 250,
  warm_start_avg_ms: 15,
  speedup_factor: 16.7,
  lazy_load_hit_rate: 0.85,
  prewarm_hit_rate: 0.72,
  checkpoints_active: 3,
  containers_prewarmed: 5,
};

// Command handler map - handlers receive the full args object
const commandHandlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  list_containers: () => mockContainers,
  get_container: (args) => mockContainers.find((c) => c.id === args?.id) || null,
  start_container: (args) => {
    const container = mockContainers.find((c) => c.id === args?.id);
    if (container) container.status = "running";
    return { success: true };
  },
  stop_container: (args) => {
    const container = mockContainers.find((c) => c.id === args?.id);
    if (container) container.status = "stopped";
    return { success: true };
  },
  remove_container: () => ({ success: true }),
  create_container: (args) => ({
    id: "new-container-id",
    name: (args as Record<string, unknown>)?.name || "new-container",
    image: (args as Record<string, unknown>)?.image,
    status: "created",
    ports: (args as Record<string, unknown>)?.ports || [],
    projectId: (args as Record<string, unknown>)?.project_id || null,
    created: new Date().toISOString(),
    startedAt: null,
    pid: null,
    rootfs: "/var/lib/hyperbox/containers/new-container-id",
    cpuPercent: 0,
    memoryMb: 0,
  }),

  list_projects: () => mockProjects,
  get_project: (args) => mockProjects.find((p) => p.id === args?.id) || null,
  start_project: () => ({ success: true }),
  stop_project: () => ({ success: true }),
  get_project_status: (args) => ({
    id: args?.id,
    status: "running",
    containers_running: 1,
    containers_stopped: 1,
    ports_in_use: [8080, 5432],
    resource_usage: {
      cpu_percent: 5.0,
      memory_mb: 256,
      disk_mb: 512,
    },
  }),

  list_images: () => mockImages,
  pull_image: () => ({ success: true }),
  remove_image: () => ({ success: true }),

  get_system_info: () => mockSystemInfo,
  get_performance_metrics: () => mockPerformanceMetrics,
  get_container_stats: () => ({
    cpu_percent: 25.5,
    memory_usage: 256000000,
    memory_limit: 512000000,
    memory_percent: 50.0,
    network_rx: 1000,
    network_tx: 500,
    block_read: 2000,
    block_write: 1000,
  }),
  get_logs: () => [
    { timestamp: new Date().toISOString(), level: "info", message: "Container started" },
    { timestamp: new Date().toISOString(), level: "info", message: "Listening on port 80" },
  ],
};

// Mock invoke function
export const invoke = vi.fn(
  async (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
    const handler = commandHandlers[cmd];
    if (handler) {
      // Pass the full args object to the handler
      return handler(args);
    }
    throw new Error(`Unknown command: ${cmd}`);
  },
);

// Event system mock
type EventCallback = (event: { payload: unknown }) => void;
const eventListeners: Map<string, Set<EventCallback>> = new Map();

export const event = {
  listen: vi.fn(async (eventName: string, callback: EventCallback): Promise<() => void> => {
    if (!eventListeners.has(eventName)) {
      eventListeners.set(eventName, new Set());
    }
    eventListeners.get(eventName)!.add(callback);
    return () => {
      eventListeners.get(eventName)?.delete(callback);
    };
  }),
  emit: vi.fn(async (eventName: string, payload: unknown) => {
    const listeners = eventListeners.get(eventName);
    if (listeners) {
      listeners.forEach((callback) => callback({ payload }));
    }
  }),
  once: vi.fn(async (eventName: string, callback: EventCallback): Promise<() => void> => {
    const wrapper = (event: { payload: unknown }) => {
      callback(event);
      eventListeners.get(eventName)?.delete(wrapper);
    };
    if (!eventListeners.has(eventName)) {
      eventListeners.set(eventName, new Set());
    }
    eventListeners.get(eventName)!.add(wrapper);
    return () => {
      eventListeners.get(eventName)?.delete(wrapper);
    };
  }),
};

// Core module mocks
export const core = {
  invoke,
};

export default {
  invoke,
  event,
  core,
};
