/**
 * Integration Tests
 *
 * Tests for cross-component interactions and workflows.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { invoke } from "../mocks/tauri";

describe("Container Lifecycle Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should complete full container lifecycle: create -> start -> stop -> remove", async () => {
    // 1. Create container
    const createResult = await invoke("create_container", {
      image: "nginx:latest",
      name: "lifecycle-test",
    });
    expect(createResult).toHaveProperty("id");
    const containerId = createResult.id;

    // 2. Start container
    const startResult = await invoke("start_container", { id: containerId });
    expect(startResult).toEqual({ success: true });

    // 3. Verify container is in list
    const containers = await invoke("list_containers", { all: true });
    expect(Array.isArray(containers)).toBe(true);

    // 4. Stop container
    const stopResult = await invoke("stop_container", { id: containerId });
    expect(stopResult).toEqual({ success: true });

    // 5. Remove container
    const removeResult = await invoke("remove_container", { id: containerId });
    expect(removeResult).toEqual({ success: true });
  });

  it("should handle container operations with project association", async () => {
    const projectId = "test-project";

    // Create container with project
    const result = await invoke("create_container", {
      image: "nginx:latest",
      name: "project-container",
      project_id: projectId,
    });

    expect(result).toHaveProperty("id");
    expect(result.projectId).toBe(projectId);
  });
});

describe("Project Lifecycle Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should get project with containers and status", async () => {
    // 1. List projects
    const projects = await invoke("list_projects");
    expect(projects.length).toBeGreaterThan(0);

    const projectId = projects[0].id;

    // 2. Get project status
    const status = await invoke("get_project_status", { id: projectId });
    expect(status).toHaveProperty("containers_running");
    expect(status).toHaveProperty("containers_stopped");
    expect(status).toHaveProperty("resource_usage");

    // 3. Start project
    const startResult = await invoke("start_project", { id: projectId });
    expect(startResult).toEqual({ success: true });

    // 4. Stop project
    const stopResult = await invoke("stop_project", { id: projectId });
    expect(stopResult).toEqual({ success: true });
  });
});

describe("Dashboard Data Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should fetch all dashboard data sources", async () => {
    // Fetch all data sources that dashboard needs
    const [systemInfo, metrics, containers] = await Promise.all([
      invoke("get_system_info"),
      invoke("get_performance_metrics"),
      invoke("list_containers", { all: false }),
    ]);

    // API returns snake_case properties
    expect(systemInfo).toHaveProperty("version");
    expect(metrics).toHaveProperty("cold_start_avg_ms");
    expect(Array.isArray(containers)).toBe(true);
  });

  it("should correlate container count with system info", async () => {
    const [systemInfo, containers] = await Promise.all([
      invoke("get_system_info"),
      invoke("list_containers", { all: true }),
    ]);

    // System info total containers should match (running + stopped)
    const totalContainers = systemInfo.containers_running + systemInfo.containers_stopped;
    expect(totalContainers).toBe(containers.length);
  });
});

describe("Image Operations Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should pull and list images", async () => {
    // 1. Get initial image count
    const initialImages = await invoke("list_images");
    const initialCount = initialImages.length;

    // 2. Pull new image
    await invoke("pull_image", { name: "redis:latest" });

    // 3. Verify list updates (in real scenario)
    const images = await invoke("list_images");
    expect(images.length).toBeGreaterThanOrEqual(initialCount);
  });
});

describe("Error Handling Integration", () => {
  it("should handle unknown commands gracefully", async () => {
    await expect(invoke("unknown_command")).rejects.toThrow("Unknown command");
  });

  it("should handle non-existent container operations", async () => {
    // Get non-existent container
    const result = await invoke("get_container", { id: "non-existent-id" });
    expect(result).toBeNull();
  });
});

describe("Real-time Updates Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should support periodic metrics fetching", async () => {
    const fetchMetrics = async () => {
      return await invoke("get_performance_metrics");
    };

    // Simulate multiple fetches
    const results = await Promise.all([fetchMetrics(), fetchMetrics(), fetchMetrics()]);

    expect(results).toHaveLength(3);
    results.forEach((result) => {
      // API returns snake_case properties
      expect(result).toHaveProperty("cold_start_avg_ms");
    });
  });

  it("should support periodic container stats fetching", async () => {
    const stats1 = await invoke("get_container_stats", { id: "container-1" });
    const stats2 = await invoke("get_container_stats", { id: "container-1" });

    expect(stats1).toEqual(stats2); // Mock returns same data
    // Stats are now returned as an object with cpu_percent, memory_usage, etc.
    expect(typeof stats1).toBe("object");
    expect(stats1).toHaveProperty("cpu_percent");
  });
});
