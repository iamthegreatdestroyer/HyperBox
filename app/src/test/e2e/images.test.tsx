/**
 * Images Page E2E Tests
 *
 * Tests for image management including listing, pulling,
 * and image details display.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import Images from "../../pages/Images";
import { invoke } from "../mocks/tauri";

function renderWithProviders(component: React.ReactNode) {
  return render(<BrowserRouter>{component}</BrowserRouter>);
}

describe("Images Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("Image Listing", () => {
    it("should render the images header", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Images/i })).toBeInTheDocument();
      });
    });

    it("should fetch images on mount", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith("list_images");
      });
    });

    it("should display image list after fetch", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        expect(screen.getByText(/nginx/i)).toBeInTheDocument();
      });
    });

    it("should show image tags", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        expect(screen.getByText(/latest/i)).toBeInTheDocument();
      });
    });
  });

  describe("Image Pull", () => {
    it("should have a pull image input or button", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        // Should have either a pull button or pull input field
        const hasInput = screen.queryByPlaceholderText(/pull/i) !== null;
        const hasButton = screen.queryByRole("button", { name: /pull/i }) !== null;
        expect(hasInput || hasButton).toBe(true);
      });
    });
  });

  describe("Image Size Display", () => {
    it("should display image sizes in readable format", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        // Size should be formatted (e.g., "142 MB" or "135.4 MB")
        // Multiple images have MB sizes, so use getAllByText
        const mbElements = screen.getAllByText(/MB/i);
        expect(mbElements.length).toBeGreaterThan(0);
      });
    });
  });

  describe("Image Actions", () => {
    it("should render images page with action buttons", async () => {
      renderWithProviders(<Images />);

      await waitFor(() => {
        // Images page should have some buttons for actions
        const buttons = screen.getAllByRole("button");
        expect(buttons.length).toBeGreaterThan(0);
      });
    });
  });
});
