import { useEffect, useState } from "react";
import { Box, Play, Square, RotateCcw, Trash2, Zap, Terminal } from "lucide-react";
import { useContainerStore, Container } from "../stores/containers";
import { clsx } from "clsx";

export default function Containers() {
  const {
    containers,
    loading,
    fetchContainers,
    startContainer,
    stopContainer,
    restartContainer,
    removeContainer,
  } = useContainerStore();
  const [showAll, setShowAll] = useState(true);
  const [selectedContainer, setSelectedContainer] = useState<Container | null>(null);

  useEffect(() => {
    fetchContainers(showAll);
  }, [fetchContainers, showAll]);

  const getStatusClass = (status: string) => {
    switch (status.toLowerCase()) {
      case "running":
        return "status-dot-running";
      case "paused":
        return "status-dot-paused";
      default:
        return "status-dot-stopped";
    }
  };

  const getBadgeClass = (status: string) => {
    switch (status.toLowerCase()) {
      case "running":
        return "badge-running";
      case "paused":
        return "badge-paused";
      default:
        return "badge-stopped";
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Containers</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Manage your running and stopped containers
          </p>
        </div>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
            <input
              type="checkbox"
              checked={showAll}
              onChange={(e) => setShowAll(e.target.checked)}
              className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
            />
            Show stopped
          </label>
        </div>
      </div>

      {/* Container Table */}
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
        ) : containers.length === 0 ? (
          <div className="p-12 text-center">
            <Box className="w-16 h-16 mx-auto mb-4 text-gray-400" />
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
              No containers found
            </h3>
            <p className="text-gray-500 dark:text-gray-400">
              Start a project or create a container to get started
            </p>
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-gray-50 dark:bg-gray-800/50">
              <tr>
                <th className="table-header">Status</th>
                <th className="table-header">Name</th>
                <th className="table-header">Image</th>
                <th className="table-header">Created</th>
                <th className="table-header">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
              {containers.map((container) => (
                <tr
                  key={container.id}
                  className={clsx(
                    "hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer",
                    selectedContainer?.id === container.id &&
                      "bg-primary-50 dark:bg-primary-900/20",
                  )}
                  onClick={() => setSelectedContainer(container)}
                >
                  <td className="table-cell">
                    <div className="flex items-center gap-2">
                      <div className={clsx("status-dot", getStatusClass(container.status))} />
                      <span className={clsx("badge", getBadgeClass(container.status))}>
                        {container.status}
                      </span>
                      {container.hasCheckpoint && (
                        <Zap className="w-4 h-4 text-warning-500" title="CRIU Checkpoint" />
                      )}
                    </div>
                  </td>
                  <td className="table-cell font-medium">
                    {container.name || container.id.slice(0, 12)}
                  </td>
                  <td className="table-cell text-gray-500 dark:text-gray-400">{container.image}</td>
                  <td className="table-cell text-gray-500 dark:text-gray-400">
                    {new Date(container.created).toLocaleDateString()}
                  </td>
                  <td className="table-cell">
                    <div className="flex items-center gap-1">
                      {container.status === "running" ? (
                        <>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              stopContainer(container.id, true);
                            }}
                            className="btn btn-ghost p-2"
                            title="Stop (with checkpoint)"
                          >
                            <Square className="w-4 h-4" />
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              restartContainer(container.id);
                            }}
                            className="btn btn-ghost p-2"
                            title="Restart"
                          >
                            <RotateCcw className="w-4 h-4" />
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            startContainer(container.id);
                          }}
                          className="btn btn-ghost p-2"
                          title={container.hasCheckpoint ? "Start (warm)" : "Start (cold)"}
                        >
                          <Play className="w-4 h-4 text-success-500" />
                        </button>
                      )}
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          removeContainer(container.id, true);
                        }}
                        className="btn btn-ghost p-2"
                        title="Remove"
                      >
                        <Trash2 className="w-4 h-4 text-error-500" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Container Details Panel */}
      {selectedContainer && (
        <div className="card">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
              Container Details
            </h2>
            <button className="btn btn-secondary">
              <Terminal className="w-4 h-4" />
              Attach
            </button>
          </div>
          <div className="p-6">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
              <div>
                <p className="text-sm text-gray-500 dark:text-gray-400">ID</p>
                <p className="font-mono text-sm text-gray-900 dark:text-white">
                  {selectedContainer.id.slice(0, 12)}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500 dark:text-gray-400">Image</p>
                <p className="font-medium text-gray-900 dark:text-white">
                  {selectedContainer.image}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500 dark:text-gray-400">Ports</p>
                <p className="font-medium text-gray-900 dark:text-white">
                  {selectedContainer.ports.length > 0 ? selectedContainer.ports.join(", ") : "None"}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500 dark:text-gray-400">Warm Start</p>
                <p className="font-medium text-gray-900 dark:text-white">
                  {selectedContainer.hasCheckpoint ? "Ready" : "Not available"}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
