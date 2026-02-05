/**
 * Dashboard Page E2E Tests
 *
 * Tests for the main dashboard page including real-time metrics,
 * container statistics, and system information display.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Dashboard from "../../pages/Dashboard";
import { invoke } from "../mocks/tauri";
import { useSystemStore } from "../../stores/system";
import { useContainerStore } from "../../stores/containers";
import { useProjectStore } from "../../stores/projects";

// Wrap component with necessary providers
function renderWithProviders(component: React.ReactNode) {
  return render(<BrowserRouter>{component}</BrowserRouter>);
}

describe("Dashboard Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Pre-initialize stores with daemonConnected: true so Dashboard makes API calls
    useSystemStore.setState({
      daemonConnected: true,
      systemInfo: {
        version: "1.0.0",
        apiVersion: "1.0",
        runtime: "hyperbox",
        os: "linux",
        arch: "x86_64",
        containersRunning: 3,
        containersPaused: 0,
        containersStopped: 1,
        images: 5,
        daemonConnected: true,
      },
      performanceMetrics: {
        coldStartAvgMs: 250,
        warmStartAvgMs: 15,
        speedupFactor: 16.7,
        lazyLoadHitRate: 0.85,
        prewarmHitRate: 0.72,
        checkpointsActive: 3,
        containersPrewarmed: 5,
      },
      loading: false,
      error: null,
    });

    useContainerStore.setState({
      containers: [
        {
          id: "abc123",
          name: "web-app",
          status: "running",
          image: "nginx:latest",
          hasCheckpoint: true,
          created: "2025-01-01T00:00:00Z",
          ports: [],
          labels: {},
        },
        {
          id: "def456",
          name: "database",
          status: "running",
          image: "postgres:15",
          hasCheckpoint: false,
          created: "2025-01-01T00:00:00Z",
          ports: [],
          labels: {},
        },
      ],
      stats: {},
      loading: false,
      error: null,
    });

    useProjectStore.setState({
      projects: [
        {
          id: "proj1",
          name: "test-project",
          path: "/app",
          status: "running",
          projectType: "compose",
          containers: [],
        },
      ],
      loading: false,
      error: null,
    });
  });

  afterEach(() => {
    // Reset stores to initial state
    useSystemStore.setState({
      daemonConnected: false,
      systemInfo: null,
      performanceMetrics: null,
      loading: false,
      error: null,
    });
    useContainerStore.setState({ containers: [], stats: {}, loading: false, error: null });
    useProjectStore.setState({ projects: [], loading: false, error: null });
  });

  describe("Initial Render", () => {
    it("should render the dashboard header", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Dashboard/i })).toBeInTheDocument();
      });
    });

    it("should display dashboard stats cards", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        // Check for stat card labels - some may appear multiple times (stat label + section header)
        const runningContainers = screen.getAllByText(/Running Containers/i);
        expect(runningContainers.length).toBeGreaterThan(0);
        expect(screen.getByText(/Active Projects/i)).toBeInTheDocument();
        expect(screen.getByText(/Total Images/i)).toBeInTheDocument();
      });
    });

    it("should fetch system info on mount", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_system_info");
      });
    });

    it("should fetch performance metrics on mount", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_performance_metrics");
      });
    });
  });

  describe("Metrics Display", () => {
    it("should display running container count", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        // Container count from store - shows "3" from containersRunning
        expect(screen.getByText("3")).toBeInTheDocument();
      });
    });

    it("should display image count", async () => {
      renderWithProviders(<Dashboard />);

      await waitFor(() => {
        // Image count from store - shows "5" from images
        expect(screen.getByText("5")).toBeInTheDocument();
      });
    });
  });

  describe("Real-time Updates", () => {
    it("should poll for metrics periodically", async () => {
      vi.useFakeTimers();

      renderWithProviders(<Dashboard />);

      // Wait for initial render and API calls
      await vi.waitFor(() => {
        expect(invoke).toHaveBeenCalled();
      });

      const initialCallCount = (invoke as ReturnType<typeof vi.fn>).mock.calls.length;

      // Advance time by 5 seconds (polling interval)
      await vi.advanceTimersByTimeAsync(5000);

      await vi.waitFor(() => {
        expect((invoke as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(
          initialCallCount,
        );
      });

      vi.useRealTimers();
    });
  });

  describe("Error Handling", () => {
    it("should handle API errors gracefully", async () => {
      (invoke as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error("Network error"));

      renderWithProviders(<Dashboard />);

      // Dashboard should still render with the pre-initialized store data
      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Dashboard/i })).toBeInTheDocument();
      });
    });
  });
});
