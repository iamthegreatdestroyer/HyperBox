import { useEffect } from "react";
import { Box, FolderKanban, Layers, Zap, Timer, Gauge } from "lucide-react";
import { useSystemStore } from "../stores/system";
import { useContainerStore } from "../stores/containers";
import { useProjectStore } from "../stores/projects";
import { clsx } from "clsx";

export default function Dashboard() {
  const { systemInfo, daemonConnected, fetchSystemInfo } = useSystemStore();
  const { containers, fetchContainers } = useContainerStore();
  const { projects, fetchProjects } = useProjectStore();

  useEffect(() => {
    if (daemonConnected) {
      fetchSystemInfo();
      fetchContainers();
      fetchProjects();
    }
  }, [daemonConnected, fetchSystemInfo, fetchContainers, fetchProjects]);

  const stats = [
    {
      name: "Running Containers",
      value: systemInfo?.containersRunning || 0,
      icon: Box,
      color: "text-success-500",
      bgColor: "bg-success-500/10",
    },
    {
      name: "Active Projects",
      value: projects.filter((p) => p.status === "running").length,
      icon: FolderKanban,
      color: "text-primary-500",
      bgColor: "bg-primary-500/10",
    },
    {
      name: "Total Images",
      value: systemInfo?.images || 0,
      icon: Layers,
      color: "text-accent-500",
      bgColor: "bg-accent-500/10",
    },
    {
      name: "Warm Start Ready",
      value: containers.filter((c) => c.hasCheckpoint).length,
      icon: Zap,
      color: "text-warning-500",
      bgColor: "bg-warning-500/10",
    },
  ];

  const runningContainers = containers.filter((c) => c.status === "running");

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
        <p className="text-gray-500 dark:text-gray-400">Overview of your container environment</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {stats.map((stat) => (
          <div key={stat.name} className="card p-6">
            <div className="flex items-center gap-4">
              <div className={clsx("p-3 rounded-xl", stat.bgColor)}>
                <stat.icon className={clsx("w-6 h-6", stat.color)} />
              </div>
              <div>
                <p className="stat-value">{stat.value}</p>
                <p className="stat-label">{stat.name}</p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Performance Highlight */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <Gauge className="w-5 h-5 text-primary-500" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Performance Metrics
          </h2>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="flex items-center gap-4">
            <Timer className="w-10 h-10 text-success-500" />
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">85ms</p>
              <p className="text-sm text-gray-500">Avg Warm Start</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <Zap className="w-10 h-10 text-warning-500" />
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">35x</p>
              <p className="text-sm text-gray-500">Faster than Docker</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <Box className="w-10 h-10 text-primary-500" />
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">47ms</p>
              <p className="text-sm text-gray-500">crun Lifecycle</p>
            </div>
          </div>
        </div>
      </div>

      {/* Running Containers */}
      <div className="card">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Running Containers
          </h2>
        </div>
        {runningContainers.length === 0 ? (
          <div className="p-8 text-center text-gray-500 dark:text-gray-400">
            <Box className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>No containers running</p>
          </div>
        ) : (
          <div className="divide-y divide-gray-200 dark:divide-gray-700">
            {runningContainers.slice(0, 5).map((container) => (
              <div key={container.id} className="container-item">
                <div className="status-dot status-dot-running" />
                <div className="flex-1">
                  <p className="font-medium text-gray-900 dark:text-white">
                    {container.name || container.id.slice(0, 12)}
                  </p>
                  <p className="text-sm text-gray-500">{container.image}</p>
                </div>
                {container.hasCheckpoint && (
                  <span className="badge badge-running">
                    <Zap className="w-3 h-3 mr-1" />
                    Warm Start
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Recent Projects */}
      <div className="card">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Recent Projects</h2>
        </div>
        {projects.length === 0 ? (
          <div className="p-8 text-center text-gray-500 dark:text-gray-400">
            <FolderKanban className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>No projects opened</p>
          </div>
        ) : (
          <div className="divide-y divide-gray-200 dark:divide-gray-700">
            {projects.slice(0, 5).map((project) => (
              <div key={project.id} className="container-item">
                <div
                  className={clsx(
                    "status-dot",
                    project.status === "running" ? "status-dot-running" : "status-dot-stopped",
                  )}
                />
                <div className="flex-1">
                  <p className="font-medium text-gray-900 dark:text-white">{project.name}</p>
                  <p className="text-sm text-gray-500">{project.path}</p>
                </div>
                <span
                  className={clsx(
                    "badge",
                    project.status === "running" ? "badge-running" : "badge-stopped",
                  )}
                >
                  {project.projectType}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
