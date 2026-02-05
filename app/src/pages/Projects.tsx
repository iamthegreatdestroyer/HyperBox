import { useEffect, useState, useCallback } from "react";
import {
  FolderKanban,
  Plus,
  Play,
  Square,
  Trash2,
  FolderOpen,
  RefreshCw,
  Cpu,
  HardDrive,
  Network,
  Container as ContainerIcon,
  FileCode,
} from "lucide-react";
import { useProjectStore, Project } from "../stores/projects";
import { useContainerStore, Container } from "../stores/containers";
import { clsx } from "clsx";
import { open } from "@tauri-apps/plugin-dialog";

// Refresh interval in milliseconds
const STATUS_REFRESH_INTERVAL = 10000;

export default function Projects() {
  const {
    projects,
    loading,
    fetchProjects,
    openProject,
    startProject,
    stopProject,
    closeProject,
    getProjectStatus,
    projectStatus,
  } = useProjectStore();
  const { containers, fetchContainers } = useContainerStore();
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);

  // Fetch projects and containers on mount
  useEffect(() => {
    fetchProjects();
    fetchContainers(true);
  }, [fetchProjects, fetchContainers]);

  // Auto-refresh project status for selected project
  useEffect(() => {
    if (!selectedProject) return;

    // Initial fetch
    getProjectStatus(selectedProject.id);

    // Set up interval
    const interval = setInterval(() => {
      getProjectStatus(selectedProject.id);
    }, STATUS_REFRESH_INTERVAL);

    return () => clearInterval(interval);
  }, [selectedProject, getProjectStatus]);

  const handleRefresh = useCallback(() => {
    fetchProjects();
    fetchContainers(true);
    if (selectedProject) {
      getProjectStatus(selectedProject.id);
    }
  }, [fetchProjects, fetchContainers, selectedProject, getProjectStatus]);

  // Get containers belonging to selected project
  const getProjectContainers = useCallback(
    (project: Project): Container[] => {
      return containers.filter((c) => project.containers.includes(c.id));
    },
    [containers],
  );

  // Format MB to human readable
  const formatMB = (mb: number): string => {
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb} MB`;
  };

  const handleOpenProject = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Project Directory",
    });

    if (selected) {
      await openProject(selected as string);
    }
  };

  const getProjectTypeIcon = (type: string) => {
    switch ((type || "").toLowerCase()) {
      case "node":
        return "🟢";
      case "python":
        return "🐍";
      case "rust":
        return "🦀";
      case "go":
        return "🐹";
      case "java":
        return "☕";
      case "dotnet":
        return "🔷";
      case "ruby":
        return "💎";
      case "php":
        return "🐘";
      default:
        return "📁";
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Projects</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Manage your development projects with isolated container environments
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={handleRefresh} className="btn btn-ghost p-2" title="Refresh">
            <RefreshCw className={clsx("w-5 h-5", loading && "animate-spin")} />
          </button>
          <button onClick={handleOpenProject} className="btn btn-primary">
            <Plus className="w-4 h-4" />
            Open Project
          </button>
        </div>
      </div>

      {/* Projects Grid */}
      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="card p-6">
              <div className="skeleton h-6 w-3/4 mb-4" />
              <div className="skeleton h-4 w-full mb-2" />
              <div className="skeleton h-4 w-1/2" />
            </div>
          ))}
        </div>
      ) : projects.length === 0 ? (
        <div className="card p-12 text-center">
          <FolderKanban className="w-16 h-16 mx-auto mb-4 text-gray-400" />
          <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
            No projects yet
          </h3>
          <p className="text-gray-500 dark:text-gray-400 mb-6">
            Open a project directory to get started with container isolation
          </p>
          <button onClick={handleOpenProject} className="btn btn-primary">
            <FolderOpen className="w-4 h-4" />
            Open Project
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((project) => (
            <div
              key={project.id}
              className={clsx(
                "card p-6 cursor-pointer transition-all hover:shadow-md",
                selectedProject?.id === project.id && "ring-2 ring-primary-500",
              )}
              onClick={() => setSelectedProject(project)}
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-3">
                  <span className="text-2xl">{getProjectTypeIcon(project.projectType)}</span>
                  <div>
                    <h3 className="font-semibold text-gray-900 dark:text-white">{project.name}</h3>
                    <span
                      className={clsx(
                        "badge",
                        project.status === "running" ? "badge-running" : "badge-stopped",
                      )}
                    >
                      {project.status}
                    </span>
                  </div>
                </div>
              </div>

              <p className="text-sm text-gray-500 dark:text-gray-400 truncate mb-4">
                {project.path}
              </p>

              <div className="flex items-center justify-between">
                <div className="text-sm text-gray-500 dark:text-gray-400">
                  {project.containers.length} containers
                </div>

                <div className="flex items-center gap-2">
                  {project.status === "running" ? (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        stopProject(project.id);
                      }}
                      className="btn btn-ghost p-2"
                      title="Stop"
                    >
                      <Square className="w-4 h-4 text-error-500" />
                    </button>
                  ) : (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        startProject(project.id);
                      }}
                      className="btn btn-ghost p-2"
                      title="Start"
                    >
                      <Play className="w-4 h-4 text-success-500" />
                    </button>
                  )}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      closeProject(project.id, true);
                    }}
                    className="btn btn-ghost p-2"
                    title="Close"
                  >
                    <Trash2 className="w-4 h-4 text-gray-400" />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Project Details Panel */}
      {selectedProject && (
        <div className="card p-6 space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Project Details</h2>
            <div className="flex items-center gap-2">
              {selectedProject.status === "running" ? (
                <button
                  onClick={() => stopProject(selectedProject.id)}
                  className="btn btn-ghost text-error-500 hover:bg-error-50 dark:hover:bg-error-900/20"
                >
                  <Square className="w-4 h-4" />
                  Stop All
                </button>
              ) : (
                <button
                  onClick={() => startProject(selectedProject.id)}
                  className="btn btn-ghost text-success-500 hover:bg-success-50 dark:hover:bg-success-900/20"
                >
                  <Play className="w-4 h-4" />
                  Start All
                </button>
              )}
            </div>
          </div>

          {/* Project Info Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Type</p>
              <p className="font-medium text-gray-900 dark:text-white flex items-center gap-2">
                <span className="text-lg">{getProjectTypeIcon(selectedProject.projectType)}</span>
                {selectedProject.projectType}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Status</p>
              <span
                className={clsx(
                  "badge",
                  selectedProject.status === "running" ? "badge-running" : "badge-stopped",
                )}
              >
                {selectedProject.status}
              </span>
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Allocated Ports</p>
              <p className="font-medium text-gray-900 dark:text-white">
                {selectedProject.ports.length > 0 ? selectedProject.ports.join(", ") : "None"}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Created</p>
              <p className="font-medium text-gray-900 dark:text-white">
                {new Date(selectedProject.created).toLocaleDateString()}
              </p>
            </div>
          </div>

          {/* Resource Usage (if available) */}
          {projectStatus[selectedProject.id] && (
            <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
              <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                Resource Usage
              </h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <Cpu className="w-5 h-5 text-primary-500" />
                  <div>
                    <p className="text-xs text-gray-500 dark:text-gray-400">CPU</p>
                    <p className="font-semibold text-gray-900 dark:text-white">
                      {projectStatus[selectedProject.id].resourceUsage.cpuPercent.toFixed(1)}%
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <HardDrive className="w-5 h-5 text-warning-500" />
                  <div>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Memory</p>
                    <p className="font-semibold text-gray-900 dark:text-white">
                      {formatMB(projectStatus[selectedProject.id].resourceUsage.memoryMb)}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <FileCode className="w-5 h-5 text-success-500" />
                  <div>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Disk</p>
                    <p className="font-semibold text-gray-900 dark:text-white">
                      {formatMB(projectStatus[selectedProject.id].resourceUsage.diskMb)}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <Network className="w-5 h-5 text-info-500" />
                  <div>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Ports in Use</p>
                    <p className="font-semibold text-gray-900 dark:text-white">
                      {projectStatus[selectedProject.id].portsInUse.length}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Container List */}
          <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
              <ContainerIcon className="w-4 h-4" />
              Containers ({selectedProject.containers.length})
            </h3>
            {selectedProject.containers.length === 0 ? (
              <p className="text-sm text-gray-500 dark:text-gray-400 italic">
                No containers in this project yet
              </p>
            ) : (
              <div className="space-y-2">
                {getProjectContainers(selectedProject).map((container) => (
                  <div
                    key={container.id}
                    className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800"
                  >
                    <div className="flex items-center gap-3">
                      <span
                        className={clsx(
                          "w-2 h-2 rounded-full",
                          container.status === "running"
                            ? "bg-success-500"
                            : container.status === "paused"
                              ? "bg-warning-500"
                              : "bg-gray-400",
                        )}
                      />
                      <div>
                        <p className="font-medium text-gray-900 dark:text-white text-sm">
                          {container.name}
                        </p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">
                          {container.image}
                        </p>
                      </div>
                    </div>
                    <span
                      className={clsx(
                        "badge text-xs",
                        container.status === "running"
                          ? "badge-running"
                          : container.status === "paused"
                            ? "badge-paused"
                            : "badge-stopped",
                      )}
                    >
                      {container.status}
                    </span>
                  </div>
                ))}
                {/* Show IDs for containers not found in container store */}
                {selectedProject.containers
                  .filter((id) => !containers.some((c) => c.id === id))
                  .map((id) => (
                    <div
                      key={id}
                      className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800"
                    >
                      <div className="flex items-center gap-3">
                        <span className="w-2 h-2 rounded-full bg-gray-400" />
                        <p className="font-mono text-xs text-gray-500 dark:text-gray-400">
                          {id.substring(0, 12)}...
                        </p>
                      </div>
                      <span className="badge badge-stopped text-xs">unknown</span>
                    </div>
                  ))}
              </div>
            )}
          </div>

          {/* Path */}
          <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-1">Project Path</p>
            <p className="font-mono text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-800 px-3 py-2 rounded-lg truncate">
              {selectedProject.path}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
