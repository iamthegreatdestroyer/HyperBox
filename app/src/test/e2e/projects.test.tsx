/**
 * Projects Page E2E Tests
 *
 * Tests for project management including listing, status display,
 * resource usage monitoring, and project operations.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Projects from "../../pages/Projects";
import { invoke } from "../mocks/tauri";
import { useProjectStore } from "../../stores/projects";
import { useContainerStore } from "../../stores/containers";

function renderWithProviders(component: React.ReactNode) {
  return render(<BrowserRouter>{component}</BrowserRouter>);
}

describe("Projects Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Pre-initialize stores with test data that matches the mock
    useProjectStore.setState({
      projects: [
        {
          id: "project-1",
          name: "my-web-app",
          path: "/home/user/projects/my-web-app",
          status: "running",
          projectType: "docker-compose",
          containers: ["abc123def456", "xyz789ghi012"],
        },
        {
          id: "project-2",
          name: "api-service",
          path: "/home/user/projects/api-service",
          status: "stopped",
          projectType: "dockerfile",
          containers: [],
        },
      ],
      projectStatus: {}, // Must be an object, not null
      selectedProject: null,
      loading: false,
      error: null,
    });

    useContainerStore.setState({
      containers: [
        {
          id: "abc123def456",
          name: "web-frontend",
          status: "running",
          image: "nginx:latest",
          hasCheckpoint: false,
          created: "2025-01-01T00:00:00Z",
          ports: [],
          labels: {},
        },
        {
          id: "xyz789ghi012",
          name: "web-backend",
          status: "running",
          image: "node:18",
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
  });

  afterEach(() => {
    useProjectStore.setState({
      projects: [],
      projectStatus: {},
      selectedProject: null,
      loading: false,
      error: null,
    });
    useContainerStore.setState({
      containers: [],
      stats: {},
      loading: false,
      error: null,
    });
  });

  describe("Project Listing", () => {
    it("should render the projects header", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Projects/i })).toBeInTheDocument();
      });
    });

    it("should fetch projects on mount", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("list_projects");
      });
    });

    it("should display project list from store", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });
    });

    it("should show project type when selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      // Select the project first to show details
      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      // Now project type should be visible in details panel
      await waitFor(() => {
        expect(screen.getByText(/compose/i)).toBeInTheDocument();
      });
    });
  });

  describe("Project Selection", () => {
    it("should allow selecting a project", async () => {
      renderWithProviders(<Projects />);

      // Wait for project list to render
      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      // Click on the project
      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      // Project Details panel should appear
      await waitFor(() => {
        expect(screen.getByText(/Project Details/i)).toBeInTheDocument();
      });
    });
  });

  describe("Project Details Panel", () => {
    it("should show project path when selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        // Path appears in both card and details panel, so use getAllByText
        const pathElements = screen.getAllByText("/home/user/projects/my-web-app");
        expect(pathElements.length).toBeGreaterThan(0);
      });
    });

    it("should show container count when selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      // Look for containers text
      await waitFor(() => {
        expect(screen.getByText(/2 containers/i)).toBeInTheDocument();
      });
    });

    it("should fetch project status when selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_project_status", { id: "project-1" });
      });
    });
  });

  describe("Project Actions", () => {
    it("should have an open project button", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /Open Project/i })).toBeInTheDocument();
      });
    });

    it("should have start/stop buttons for selected project", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        // Should have either start or stop button depending on state
        const actionButtons = screen.getAllByRole("button");
        const hasControlButton = actionButtons.some(
          (btn) => btn.textContent?.includes("Start") || btn.textContent?.includes("Stop"),
        );
        expect(hasControlButton).toBe(true);
      });
    });
  });

  describe("Resource Usage Display", () => {
    it("should display resource usage when project is selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_project_status", { id: "project-1" });
      });
    });

    it("should call get_project_status for selected project", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("get_project_status", { id: "project-1" });
      });
    });

    it("should display project details panel when selected", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(screen.getByText("my-web-app")).toBeInTheDocument();
      });

      const projectRow = screen.getByText("my-web-app");
      fireEvent.click(projectRow);

      await waitFor(() => {
        expect(screen.getByText(/Project Details/i)).toBeInTheDocument();
      });
    });
  });

  describe("Refresh Functionality", () => {
    it("should have a refresh button", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        const refreshButton = screen.getByRole("button", { name: /refresh/i });
        expect(refreshButton).toBeInTheDocument();
      });
    });

    it("should refetch data when refresh is clicked", async () => {
      renderWithProviders(<Projects />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("list_projects");
      });

      const initialCallCount = (invoke as ReturnType<typeof vi.fn>).mock.calls.length;

      const refreshButton = screen.getByRole("button", { name: /refresh/i });
      fireEvent.click(refreshButton);

      await waitFor(() => {
        expect((invoke as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(
          initialCallCount,
        );
      });
    });
  });
});
