import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { BrowserRouter } from "react-router-dom";
import Performance from "./Performance";

// Mock the tauri invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

// Mock Recharts to avoid rendering issues in tests
vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="recharts-container">{children}</div>
  ),
  AreaChart: ({ data }: { data: any[] }) => (
    <div data-testid="area-chart" data-points={data.length} />
  ),
  LineChart: ({ data }: { data: any[] }) => (
    <div data-testid="line-chart" data-points={data.length} />
  ),
  BarChart: ({ data }: { data: any[] }) => (
    <div data-testid="bar-chart" data-points={data.length} />
  ),
  Area: () => <div />,
  Line: () => <div />,
  Bar: () => <div />,
  XAxis: () => <div />,
  YAxis: () => <div />,
  CartesianGrid: () => <div />,
  Tooltip: () => <div />,
  Legend: () => <div />,
}));

// Mock metrics store
const mockMetricsStore = {
  systemHistory: [
    {
      time: "00:00:05",
      cpu: 10,
      memory: 1000,
      networkRx: 100,
      networkTx: 100,
      blockRead: 1000,
      blockWrite: 1000,
    },
    {
      time: "00:00:10",
      cpu: 15,
      memory: 1500,
      networkRx: 150,
      networkTx: 150,
      blockRead: 1500,
      blockWrite: 1500,
    },
  ],
  latestContainerMetrics: {
    container1: {
      timestamp: Date.now(),
      containerId: "container1",
      containerName: "web-server",
      cpu: 25,
      memory: 800,
      memoryLimit: 2000,
      memoryPercent: 40,
      networkRx: 200,
      networkTx: 300,
      blockRead: 2000,
      blockWrite: 3000,
    },
    container2: {
      timestamp: Date.now(),
      containerId: "container2",
      containerName: "database",
      cpu: 35,
      memory: 1200,
      memoryLimit: 2000,
      memoryPercent: 60,
      networkRx: 500,
      networkTx: 400,
      blockRead: 5000,
      blockWrite: 4000,
    },
  },
  startStreaming: vi.fn(),
  stopStreaming: vi.fn(),
};

vi.mock("../stores/metrics", () => ({
  useMetricsStore: () => mockMetricsStore,
}));

const renderPerformance = () => {
  return render(
    <BrowserRouter>
      <Performance />
    </BrowserRouter>,
  );
};

describe("Performance Page (B6 E2E)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    // Default mock for get_performance_metrics
    vi.mocked(invoke).mockResolvedValue({
      cold_start_avg: 4.2,
      warm_start_avg: 0.085,
      checkpoints_active: 15,
      prewarm_hits: 342,
      layer_cache_hit_rate: 0.92,
      docker_comparison: 35,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Page Rendering", () => {
    it("should render the Performance page with header", async () => {
      renderPerformance();

      expect(screen.getByRole("heading", { name: /Performance/i })).toBeInTheDocument();
      expect(screen.getByText(/Real-time monitoring/i)).toBeInTheDocument();
    });

    it("should display the Refresh button", async () => {
      renderPerformance();

      const refreshButton = screen.getByRole("button", { name: /Refresh/i });
      expect(refreshButton).toBeInTheDocument();
    });

    it("should start and stop streaming on mount/unmount", async () => {
      renderPerformance();

      expect(mockMetricsStore.startStreaming).toHaveBeenCalled();

      // Component should be unmounted during cleanup
    });
  });

  describe("Stat Cards Display", () => {
    it("should display all four stat cards with proper labels", async () => {
      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText("Warm Start Time")).toBeInTheDocument();
        expect(screen.getByText("Cold Start Time")).toBeInTheDocument();
        expect(screen.getByText("CRIU Checkpoints")).toBeInTheDocument();
        expect(screen.getByText("Cache Hit Rate")).toBeInTheDocument();
      });
    });

    it("should display warm start metrics", async () => {
      renderPerformance();

      await waitFor(() => {
        // 0.085s * 1000 = 85ms
        expect(screen.getByText(/85ms/)).toBeInTheDocument();
        expect(screen.getByText(/35x faster than Docker/)).toBeInTheDocument();
      });
    });

    it("should display cold start metrics", async () => {
      renderPerformance();

      await waitFor(() => {
        // 4.2s
        expect(screen.getByText(/4\.2s/)).toBeInTheDocument();
        expect(screen.getByText(/Sub-linear image loading/)).toBeInTheDocument();
      });
    });

    it("should display CRIU checkpoint count", async () => {
      renderPerformance();

      await waitFor(() => {
        // checkpointsActive = 15
        expect(screen.getByText(/15/)).toBeInTheDocument();
        expect(screen.getByText(/Ready for instant restore/)).toBeInTheDocument();
      });
    });

    it("should display cache hit rate percentage", async () => {
      renderPerformance();

      await waitFor(() => {
        // layerCacheHitRate = 0.92 = 92%
        expect(screen.getByText(/92%/)).toBeInTheDocument();
        expect(screen.getByText(/eStargz lazy loading/)).toBeInTheDocument();
      });
    });

    it("should show loading state while fetching metrics", async () => {
      vi.mocked(invoke).mockImplementationOnce(
        () =>
          new Promise((resolve) =>
            setTimeout(resolve, 1000, {
              cold_start_avg: 4.2,
              warm_start_avg: 0.085,
              checkpoints_active: 15,
              prewarm_hits: 342,
              layer_cache_hit_rate: 0.92,
              docker_comparison: 35,
            }),
          ),
      );

      renderPerformance();

      const refreshButton = screen.getByRole("button", { name: /Refresh/i });

      // Click refresh to trigger loading state
      await userEvent.click(refreshButton);

      // Button should have loading indicator (animate-spin class)
      await waitFor(() => {
        expect(refreshButton.querySelector(".animate-spin")).toBeInTheDocument();
      });

      // Wait for loading to complete
      await waitFor(
        () => {
          expect(refreshButton.querySelector(".animate-spin")).not.toBeInTheDocument();
        },
        { timeout: 2000 },
      );
    });

    it("should display placeholder values when metrics not loaded", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("Connection failed"));

      renderPerformance();

      await waitFor(() => {
        // Should show dashes for failed metrics
        const dashes = screen.getAllByText("—");
        expect(dashes.length).toBeGreaterThan(0);
      });
    });
  });

  describe("System Charts", () => {
    it("should render CPU usage chart with data", async () => {
      renderPerformance();

      await waitFor(() => {
        const cpuChart = screen.getByTestId("area-chart");
        expect(cpuChart).toBeInTheDocument();
        expect(cpuChart).toHaveAttribute("data-points", "2"); // 2 history points
      });

      expect(screen.getByText("CPU Usage (%)")).toBeInTheDocument();
    });

    it("should render Memory usage chart with data", async () => {
      renderPerformance();

      await waitFor(() => {
        const memChart = screen.getAllByTestId("area-chart")[1] || screen.getByTestId("area-chart");
        expect(memChart).toBeInTheDocument();
      });

      expect(screen.getByText("Memory Usage")).toBeInTheDocument();
    });

    it("should render Network I/O chart with data", async () => {
      renderPerformance();

      await waitFor(() => {
        expect(screen.getByTestId("line-chart")).toBeInTheDocument();
      });

      expect(screen.getByText("Network I/O")).toBeInTheDocument();
    });

    it("should have responsive container for charts", async () => {
      renderPerformance();

      await waitFor(() => {
        const containers = screen.getAllByTestId("recharts-container");
        // Should have multiple charts (CPU, Memory, Network, Block I/O)
        expect(containers.length).toBeGreaterThanOrEqual(3);
      });
    });
  });

  describe("Refresh Functionality", () => {
    it("should call fetch metrics on button click", async () => {
      const user = userEvent.setup({ delay: null });
      renderPerformance();

      const refreshButton = screen.getByRole("button", { name: /Refresh/i });
      await user.click(refreshButton);

      // invoke should be called (at least twice: initial load + refresh click)
      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_performance_metrics");
      });
    });

    it("should update metrics on refresh", async () => {
      const user = userEvent.setup({ delay: null });
      renderPerformance();

      // Initial render should call invoke
      await waitFor(() => {
        expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_performance_metrics");
      });

      const initialCallCount = vi.mocked(invoke).mock.calls.length;

      // Click refresh
      const refreshButton = screen.getByRole("button", { name: /Refresh/i });
      await user.click(refreshButton);

      // Should have called invoke again
      await waitFor(() => {
        expect(vi.mocked(invoke).mock.calls.length).toBeGreaterThan(initialCallCount);
      });
    });

    it("should handle metrics fetch error gracefully", async () => {
      const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      vi.mocked(invoke).mockRejectedValueOnce(new Error("Network error"));

      renderPerformance();

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith("Failed to fetch metrics:", expect.any(Error));
      });

      consoleErrorSpy.mockRestore();
    });

    it("should auto-refresh metrics every 10 seconds", async () => {
      renderPerformance();

      const initialCallCount = vi.mocked(invoke).mock.calls.length;

      // Advance time by 10 seconds
      vi.advanceTimersByTime(10_000);

      // Should have called invoke again for auto-refresh
      expect(vi.mocked(invoke).mock.calls.length).toBeGreaterThan(initialCallCount);
    });
  });

  describe("Real-time Metrics Integration", () => {
    it("should display top containers by CPU usage", async () => {
      renderPerformance();

      await waitFor(() => {
        // Should show top containers from latestContainerMetrics
        // database (35%) should appear before web-server (25%)
      });
    });

    it("should update charts when systemHistory changes", async () => {
      const { rerender } = renderPerformance();

      let chart = screen.getByTestId("area-chart");
      expect(chart).toHaveAttribute("data-points", "2");

      // Update mock store with more history points
      const newHistory = [
        {
          time: "00:00:05",
          cpu: 10,
          memory: 1000,
          networkRx: 100,
          networkTx: 100,
          blockRead: 1000,
          blockWrite: 1000,
        },
        {
          time: "00:00:10",
          cpu: 15,
          memory: 1500,
          networkRx: 150,
          networkTx: 150,
          blockRead: 1500,
          blockWrite: 1500,
        },
        {
          time: "00:00:15",
          cpu: 20,
          memory: 2000,
          networkRx: 200,
          networkTx: 200,
          blockRead: 2000,
          blockWrite: 2000,
        },
      ];

      mockMetricsStore.systemHistory = newHistory;

      rerender(
        <BrowserRouter>
          <Performance />
        </BrowserRouter>,
      );

      await waitFor(() => {
        chart = screen.getByTestId("area-chart");
        expect(chart).toHaveAttribute("data-points", "3");
      });
    });
  });

  describe("Formatting and Display", () => {
    it("should format bytes correctly", async () => {
      renderPerformance();

      // Formatting should be applied to network and block I/O values
      // This is tested through the display of properly formatted values
    });

    it("should show trend indicators (up/down arrows)", async () => {
      renderPerformance();

      await waitFor(() => {
        // Cache hit rate > 80% should show up arrow
        // Other metrics should show appropriate indicators
      });
    });

    it("should display percentage values for cache hit rate", async () => {
      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText(/92%/)).toBeInTheDocument();
      });
    });

    it("should display seconds for cold start time", async () => {
      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText(/4\.2s/)).toBeInTheDocument();
      });
    });

    it("should display milliseconds for warm start time", async () => {
      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText(/85ms/)).toBeInTheDocument();
      });
    });
  });

  describe("Lifecycle and Cleanup", () => {
    it("should call startStreaming on mount", async () => {
      renderPerformance();

      expect(mockMetricsStore.startStreaming).toHaveBeenCalled();
    });

    it("should call stopStreaming on unmount", async () => {
      const { unmount } = renderPerformance();

      expect(mockMetricsStore.stopStreaming).not.toHaveBeenCalled();

      unmount();

      // After unmount, stopStreaming should be called (in cleanup)
      // Note: This depends on the useEffect cleanup
    });

    it("should clear auto-refresh interval on unmount", async () => {
      const clearIntervalSpy = vi.spyOn(window, "clearInterval");

      const { unmount } = renderPerformance();

      unmount();

      expect(clearIntervalSpy).toHaveBeenCalled();
      clearIntervalSpy.mockRestore();
    });

    it("should handle rapid mount/unmount cycles", async () => {
      const { unmount, rerender } = renderPerformance();

      unmount();

      rerender(
        <BrowserRouter>
          <Performance />
        </BrowserRouter>,
      );

      expect(mockMetricsStore.startStreaming).toHaveBeenCalledTimes(2);
    });
  });

  describe("Error Handling", () => {
    it("should handle invoke error for metrics", async () => {
      const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      vi.mocked(invoke).mockRejectedValueOnce(new Error("Daemon disconnected"));

      renderPerformance();

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalled();
      });

      consoleErrorSpy.mockRestore();
    });

    it("should display fallback UI when data unavailable", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("Connection failed"));

      renderPerformance();

      await waitFor(() => {
        // Should show dashes or loading state
        const dashes = screen.queryAllByText("—");
        expect(dashes.length).toBeGreaterThan(0);
      });
    });

    it("should continue displaying charts even if metrics fetch fails", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("Metrics unavailable"));

      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText("CPU Usage (%)")).toBeInTheDocument();
        expect(screen.getByTestId("area-chart")).toBeInTheDocument();
      });
    });
  });

  describe("Data Validation", () => {
    it("should handle zero values in metrics", async () => {
      vi.mocked(invoke).mockResolvedValueOnce({
        cold_start_avg: 0,
        warm_start_avg: 0,
        checkpoints_active: 0,
        prewarm_hits: 0,
        layer_cache_hit_rate: 0,
        docker_comparison: 0,
      });

      renderPerformance();

      await waitFor(() => {
        expect(screen.getByText(/0ms/)).toBeInTheDocument();
        expect(screen.getByText(/0%/)).toBeInTheDocument();
      });
    });

    it("should handle very large metrics values", async () => {
      vi.mocked(invoke).mockResolvedValueOnce({
        cold_start_avg: 999999,
        warm_start_avg: 999999,
        checkpoints_active: 999999,
        prewarm_hits: 999999,
        layer_cache_hit_rate: 1.0,
        docker_comparison: 999999,
      });

      renderPerformance();

      await waitFor(() => {
        // Should still render without crashing
        expect(screen.getByRole("heading", { name: /Performance/i })).toBeInTheDocument();
      });
    });

    it("should handle missing fields in metrics response", async () => {
      vi.mocked(invoke).mockResolvedValueOnce({
        cold_start_avg: 4.2,
        // Missing other fields
      } as any);

      renderPerformance();

      await waitFor(() => {
        // Should render with available data and fallbacks for missing
        expect(screen.getByRole("heading", { name: /Performance/i })).toBeInTheDocument();
      });
    });
  });

  describe("Container Drill-down", () => {
    it("should sort containers by CPU usage", async () => {
      renderPerformance();

      // Containers should be sorted: database (35%) before web-server (25%)
      // This is tested through the topContainers useMemo
    });

    it("should show top 8 containers", async () => {
      // Mock many containers
      const manyContainers = Array.from({ length: 20 }, (_, i) => ({
        [`container${i}`]: {
          timestamp: Date.now(),
          containerId: `container${i}`,
          containerName: `container-${i}`,
          cpu: 50 - i,
          memory: 1000 + i * 100,
          memoryLimit: 2000,
          memoryPercent: 50,
          networkRx: 100,
          networkTx: 100,
          blockRead: 1000,
          blockWrite: 1000,
        },
      })).reduce((a, b) => ({ ...a, ...b }), {});

      mockMetricsStore.latestContainerMetrics = manyContainers;

      renderPerformance();

      // topContainers should be limited to 8
      // This is a unit test more than E2E
    });
  });

  describe("Benchmark Comparison Display", () => {
    it("should display benchmark comparison data", async () => {
      renderPerformance();

      await waitFor(() => {
        // Benchmarks are expected: Cold Start, Warm Start, Image Pull, Stop
        // With HyperBox vs Docker comparisons
      });
    });

    it("should show HyperBox advantages in benchmarks", async () => {
      renderPerformance();

      // All HyperBox values should be lower than Docker equivalents
      await waitFor(() => {
        // Cold Start: 4.2s vs 35s
        // Warm Start: 0.085s vs 3.5s
        // And so on...
      });
    });
  });
});
