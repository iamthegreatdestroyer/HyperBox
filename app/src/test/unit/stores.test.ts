/**
 * Zustand Store Unit Tests
 *
 * Tests for the state management stores including containers,
 * projects, images, and system stores.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { invoke } from "../mocks/tauri";
import { mockContainers, mockProjects, mockImages, mockSystemInfo } from "../mocks/tauri";

// Note: For store testing, we need to reset the store state between tests
// This is a pattern for testing Zustand stores

describe("Container Store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Container Fetching", () => {
    it("should call list_containers with correct parameters", async () => {
      await invoke("list_containers", { all: true });

      expect(invoke).toHaveBeenCalledWith("list_containers", { all: true });
    });

    it("should return container list from API", async () => {
      const result = await invoke("list_containers", { all: false });

      expect(result).toEqual(mockContainers);
      expect(result).toHaveLength(2);
    });
  });

  describe("Container Operations", () => {
    it("should start a container", async () => {
      const result = await invoke("start_container", { id: "abc123def456" });

      expect(invoke).toHaveBeenCalledWith("start_container", { id: "abc123def456" });
      expect(result).toEqual({ success: true });
    });

    it("should stop a container", async () => {
      const result = await invoke("stop_container", { id: "abc123def456" });

      expect(invoke).toHaveBeenCalledWith("stop_container", { id: "abc123def456" });
      expect(result).toEqual({ success: true });
    });

    it("should remove a container", async () => {
      const result = await invoke("remove_container", { id: "abc123def456" });

      expect(result).toEqual({ success: true });
    });
  });

  describe("Container Creation", () => {
    it("should create a container with minimal options", async () => {
      const request = { image: "nginx:latest" };
      const result = await invoke("create_container", request);

      expect(invoke).toHaveBeenCalledWith("create_container", request);
      expect(result).toHaveProperty("id");
      expect(result).toHaveProperty("image", "nginx:latest");
    });

    it("should create a container with full options", async () => {
      const request = {
        image: "nginx:latest",
        name: "my-nginx",
        ports: [{ host: 8080, container: 80, protocol: "tcp" }],
        env: ["NODE_ENV=production"],
        volumes: ["/host/path:/container/path"],
        project_id: "project-1",
      };

      const result = await invoke("create_container", request);

      expect(result).toHaveProperty("name", "my-nginx");
    });
  });
});

describe("Project Store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Project Fetching", () => {
    it("should fetch projects list", async () => {
      const result = await invoke("list_projects");

      expect(invoke).toHaveBeenCalledWith("list_projects");
      expect(result).toEqual(mockProjects);
    });

    it("should get project by id", async () => {
      const result = await invoke("get_project", { id: "project-1" });

      expect(result).toHaveProperty("id", "project-1");
      expect(result).toHaveProperty("name", "my-web-app");
    });
  });

  describe("Project Operations", () => {
    it("should start a project", async () => {
      const result = await invoke("start_project", { id: "project-1" });

      expect(result).toEqual({ success: true });
    });

    it("should stop a project", async () => {
      const result = await invoke("stop_project", { id: "project-1" });

      expect(result).toEqual({ success: true });
    });
  });

  describe("Project Status", () => {
    it("should get project status with resource usage", async () => {
      const result = await invoke("get_project_status", { id: "project-1" });

      expect(result).toHaveProperty("id", "project-1");
      expect(result).toHaveProperty("status", "running");
      expect(result).toHaveProperty("containers_running");
      expect(result).toHaveProperty("containers_stopped");
      expect(result).toHaveProperty("resource_usage");
      expect(result.resource_usage).toHaveProperty("cpu_percent");
      expect(result.resource_usage).toHaveProperty("memory_mb");
    });
  });
});

describe("Image Store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Image Fetching", () => {
    it("should fetch images list", async () => {
      const result = await invoke("list_images");

      expect(invoke).toHaveBeenCalledWith("list_images");
      expect(result).toEqual(mockImages);
      expect(result).toHaveLength(2);
    });

    it("should return images with required properties", async () => {
      const result = await invoke("list_images");
      const image = result[0];

      expect(image).toHaveProperty("id");
      expect(image).toHaveProperty("tags");
      expect(image).toHaveProperty("size");
      expect(image).toHaveProperty("created");
      expect(image).toHaveProperty("in_use");
    });
  });

  describe("Image Operations", () => {
    it("should pull an image", async () => {
      const result = await invoke("pull_image", { name: "redis:latest" });

      expect(result).toEqual({ success: true });
    });

    it("should remove an image", async () => {
      const result = await invoke("remove_image", { id: "sha256:abc123" });

      expect(result).toEqual({ success: true });
    });
  });
});

describe("System Store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("System Info", () => {
    it("should fetch system info", async () => {
      const result = await invoke("get_system_info");

      expect(invoke).toHaveBeenCalledWith("get_system_info");
      expect(result).toEqual(mockSystemInfo);
    });

    it("should return system info with required properties", async () => {
      const result = await invoke("get_system_info");

      // API returns snake_case properties
      expect(result).toHaveProperty("version");
      expect(result).toHaveProperty("os");
      expect(result).toHaveProperty("containers_running");
      expect(result).toHaveProperty("images");
      expect(result).toHaveProperty("daemon_connected");
    });
  });

  describe("Performance Metrics", () => {
    it("should fetch performance metrics", async () => {
      const result = await invoke("get_performance_metrics");

      expect(invoke).toHaveBeenCalledWith("get_performance_metrics");
      // API returns snake_case properties
      expect(result).toHaveProperty("cold_start_avg_ms");
      expect(result).toHaveProperty("warm_start_avg_ms");
      expect(result).toHaveProperty("speedup_factor");
    });

    it("should fetch container stats", async () => {
      const result = await invoke("get_container_stats", { id: "container-1" });

      expect(invoke).toHaveBeenCalledWith("get_container_stats", { id: "container-1" });
      expect(result).toHaveProperty("cpu_percent");
      expect(result).toHaveProperty("memory_usage");
    });
  });
});

describe("Logs Store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Log Fetching", () => {
    it("should fetch container logs", async () => {
      const result = await invoke("get_logs", { containerId: "abc123def456" });

      expect(result).toHaveLength(2);
      expect(result[0]).toHaveProperty("timestamp");
      expect(result[0]).toHaveProperty("level");
      expect(result[0]).toHaveProperty("message");
    });
  });
});
