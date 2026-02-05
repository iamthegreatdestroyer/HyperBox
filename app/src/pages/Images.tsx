import { useEffect, useState } from "react";
import { Layers, Download, Trash2, Search, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { clsx } from "clsx";

interface Image {
  id: string;
  tags: string[];
  size: number;
  created: string;
  inUse: boolean;
}

export default function Images() {
  const [images, setImages] = useState<Image[]>([]);
  const [loading, setLoading] = useState(false);
  const [pullImage, setPullImage] = useState("");
  const [pulling, setPulling] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");

  const fetchImages = async () => {
    setLoading(true);
    try {
      const rawImages = await invoke<
        {
          id: string;
          tags: string[];
          size: number;
          created: string;
          in_use: boolean;
        }[]
      >("list_images");

      setImages(
        rawImages.map((img) => ({
          id: img.id,
          tags: img.tags,
          size: img.size,
          created: img.created,
          inUse: img.in_use,
        })),
      );
    } catch (error) {
      console.error("Failed to fetch images:", error);
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchImages();
  }, []);

  const handlePull = async () => {
    if (!pullImage.trim()) return;
    setPulling(true);
    try {
      await invoke("pull_image", { reference: pullImage });
      setPullImage("");
      fetchImages();
    } catch (error) {
      console.error("Failed to pull image:", error);
    }
    setPulling(false);
  };

  const handleRemove = async (id: string) => {
    try {
      await invoke("remove_image", { id, force: false });
      fetchImages();
    } catch (error) {
      console.error("Failed to remove image:", error);
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const filteredImages = images.filter((img) =>
    (img.tags || []).some((tag) => tag.toLowerCase().includes(searchTerm.toLowerCase())),
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Images</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Manage container images with lazy loading optimization
          </p>
        </div>
        <button onClick={fetchImages} className="btn btn-secondary">
          <RefreshCw className={clsx("w-4 h-4", loading && "animate-spin")} />
          Refresh
        </button>
      </div>

      {/* Pull Image */}
      <div className="card p-4">
        <div className="flex gap-4">
          <div className="flex-1 relative">
            <Download className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
            <input
              type="text"
              placeholder="Pull image (e.g., nginx:latest)"
              value={pullImage}
              onChange={(e) => setPullImage(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handlePull()}
              className="input pl-10 w-full"
            />
          </div>
          <button onClick={handlePull} disabled={pulling} className="btn btn-primary">
            {pulling ? "Pulling..." : "Pull"}
          </button>
        </div>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
        <input
          type="text"
          placeholder="Search images..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="input pl-10 w-full"
        />
      </div>

      {/* Images Table */}
      <div className="card overflow-hidden">
        {loading ? (
          <div className="p-8">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="flex gap-4 mb-4">
                <div className="skeleton w-12 h-12 rounded-lg" />
                <div className="flex-1">
                  <div className="skeleton h-5 w-1/3 mb-2" />
                  <div className="skeleton h-4 w-1/2" />
                </div>
              </div>
            ))}
          </div>
        ) : filteredImages.length === 0 ? (
          <div className="p-12 text-center">
            <Layers className="w-16 h-16 mx-auto mb-4 text-gray-400" />
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
              No images found
            </h3>
            <p className="text-gray-500 dark:text-gray-400">Pull an image to get started</p>
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-gray-50 dark:bg-gray-800/50">
              <tr>
                <th className="table-header">Repository</th>
                <th className="table-header">Tag</th>
                <th className="table-header">Size</th>
                <th className="table-header">Created</th>
                <th className="table-header">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
              {filteredImages.map((image) =>
                image.tags.map((tag, index) => {
                  const [repo, tagName] = tag.split(":");
                  return (
                    <tr
                      key={`${image.id}-${index}`}
                      className="hover:bg-gray-50 dark:hover:bg-gray-800/50"
                    >
                      <td className="table-cell font-medium">{repo || "<none>"}</td>
                      <td className="table-cell">
                        <span className="badge badge-default">{tagName || "latest"}</span>
                      </td>
                      <td className="table-cell text-gray-500 dark:text-gray-400">
                        {formatSize(image.size)}
                      </td>
                      <td className="table-cell text-gray-500 dark:text-gray-400">
                        {new Date(image.created).toLocaleDateString()}
                      </td>
                      <td className="table-cell">
                        <button
                          onClick={() => handleRemove(image.id)}
                          disabled={image.inUse}
                          className={clsx(
                            "btn btn-ghost p-2",
                            image.inUse && "opacity-50 cursor-not-allowed",
                          )}
                          title={image.inUse ? "Image in use" : "Remove"}
                        >
                          <Trash2 className="w-4 h-4 text-error-500" />
                        </button>
                      </td>
                    </tr>
                  );
                }),
              )}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
