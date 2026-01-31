import { useEffect, useState } from "react";
import { FolderKanban, Plus, Play, Square, Trash2, FolderOpen } from "lucide-react";
import { useProjectStore, Project } from "../stores/projects";
import { clsx } from "clsx";
import { open } from "@tauri-apps/plugin-dialog";

export default function Projects() {
  const { projects, loading, fetchProjects, openProject, startProject, stopProject, closeProject } =
    useProjectStore();
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);

  useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

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
    switch (type.toLowerCase()) {
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
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Projects</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Manage your development projects with isolated container environments
          </p>
        </div>
        <button onClick={handleOpenProject} className="btn btn-primary">
          <Plus className="w-4 h-4" />
          Open Project
        </button>
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
        <div className="card p-6">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Project Details
          </h2>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Type</p>
              <p className="font-medium text-gray-900 dark:text-white">
                {selectedProject.projectType}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400">Containers</p>
              <p className="font-medium text-gray-900 dark:text-white">
                {selectedProject.containers.length}
              </p>
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
        </div>
      )}
    </div>
  );
}
