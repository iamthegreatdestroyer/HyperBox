/**
 * Containers Page E2E Tests
 *
 * Tests for container management including listing, creation,
 * start/stop operations, and container details.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Containers from "../../pages/Containers";
import CreateContainerModal from "../../components/CreateContainerModal";
import { invoke } from "../mocks/tauri";
import { useContainerStore } from "../../stores/containers";

function renderWithProviders(component: React.ReactNode) {
  return render(<BrowserRouter>{component}</BrowserRouter>);
}

describe("Containers Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Pre-initialize containers store with test data
    useContainerStore.setState({
      containers: [
        {
          id: "abc123",
          name: "web-app",
          status: "running",
          image: "nginx:latest",
          hasCheckpoint: true,
          created: "2025-01-01T00:00:00Z",
          ports: ["8080:80/tcp"],
          labels: {},
        },
        {
          id: "def456",
          name: "database",
          status: "stopped",
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
  });

  afterEach(() => {
    useContainerStore.setState({
      containers: [],
      stats: {},
      loading: false,
      error: null,
    });
  });

  describe("Container Listing", () => {
    it("should render the containers header", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Containers/i })).toBeInTheDocument();
      });
    });

    it("should fetch containers on mount", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("list_containers", expect.any(Object));
      });
    });

    it("should display container list from store", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        // Container names from mock data should appear
        expect(screen.getByText("web-server")).toBeInTheDocument();
      });
    });

    it("should show container status badges", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(screen.getByText(/running/i)).toBeInTheDocument();
      });
    });

    it("should display container images", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(screen.getByText("nginx:latest")).toBeInTheDocument();
      });
    });
  });

  describe("Container Actions", () => {
    it("should have a create container button", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /Create Container/i })).toBeInTheDocument();
      });
    });

    it("should open create modal when clicking create button", async () => {
      renderWithProviders(<Containers />);

      const createButton = await screen.findByRole("button", { name: /Create Container/i });
      fireEvent.click(createButton);

      await waitFor(() => {
        // Modal header should appear
        expect(screen.getByRole("heading", { name: /Create Container/i })).toBeInTheDocument();
      });
    });
  });

  describe("Container Row Actions", () => {
    it("should show action buttons for each container", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        // Look for action buttons (stop, restart, delete, etc.)
        const buttons = screen.getAllByRole("button");
        expect(buttons.length).toBeGreaterThan(1);
      });
    });
  });

  describe("Filtering", () => {
    it("should have a show stopped checkbox", async () => {
      renderWithProviders(<Containers />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Show stopped/i)).toBeInTheDocument();
      });
    });
  });
});

describe("CreateContainerModal", () => {
  const mockOnClose = vi.fn();
  const mockOnSuccess = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render modal when open", () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    // Use heading role to distinguish from button text
    expect(screen.getByRole("heading", { name: /Create Container/i })).toBeInTheDocument();
  });

  it("should not render modal when closed", () => {
    render(<CreateContainerModal isOpen={false} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    expect(screen.queryByText(/Create Container/i)).not.toBeInTheDocument();
  });

  it("should have image input field", () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    // Label doesn't have htmlFor, so use placeholder text
    expect(screen.getByPlaceholderText(/nginx:latest/i)).toBeInTheDocument();
  });

  it("should have container name input field", () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    // Label doesn't have htmlFor, so use placeholder text
    expect(screen.getByPlaceholderText(/my-web-server/i)).toBeInTheDocument();
  });

  it("should have tabs for different settings", () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    expect(screen.getByText(/Basic/i)).toBeInTheDocument();
    expect(screen.getByText(/Environment/i)).toBeInTheDocument();
    expect(screen.getByText(/Ports/i)).toBeInTheDocument();
    expect(screen.getByText(/Volumes/i)).toBeInTheDocument();
  });

  it("should call onClose when cancel button clicked", () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    const cancelButton = screen.getByRole("button", { name: /cancel/i });
    fireEvent.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it("should require image field before submission", async () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    const createButton = screen.getByRole("button", { name: /create container/i });

    // Image field should be required
    expect(createButton).toBeDisabled();
  });

  it("should enable submit when image is provided", async () => {
    render(<CreateContainerModal isOpen={true} onClose={mockOnClose} onSuccess={mockOnSuccess} />);

    // Label doesn't have htmlFor, so use placeholder text
    const imageInput = screen.getByPlaceholderText(/nginx:latest/i);
    fireEvent.change(imageInput, { target: { value: "nginx:latest" } });

    const createButton = screen.getByRole("button", { name: /create container/i });
    expect(createButton).not.toBeDisabled();
  });
});
