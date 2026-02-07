import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Box,
  FolderKanban,
  Layers,
  Zap,
  Timer,
  Gauge,
  RefreshCw,
  Cpu,
  HardDrive,
  Activity,
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Network,
  MemoryStick,
} from "lucide-react";
import { clsx } from "clsx";
import {
  AreaChart,
  Area,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { useSystemStore } from "../stores/system";
import { useContainerStore } from "../stores/containers";
import { useProjectStore } from "../stores/projects";
import { useMetricsStore, type MemoryPressure } from "../stores/metrics";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function formatBytesShort(bytes: number): string {
  if (bytes === 0) return "0";
  const k = 1024;
  const sizes = ["B", "K", "M", "G"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(0)}${sizes[i]}`;
}

const CHART_TOOLTIP_STYLE = {
  backgroundColor: "#1F2937",
  border: "none",
  borderRadius: "8px",
  fontSize: 12,
};

const PRESSURE_COLORS: Record<MemoryPressure["level"], string> = {
  low: "text-success-500 bg-success-500/10",
  moderate: "text-warning-500 bg-warning-500/10",
  high: "text-orange-500 bg-orange-500/10",
  critical: "text-error-500 bg-error-500/10",
};

const PRESSURE_LABELS: Record<MemoryPressure["level"], string> = {
  low: "Low",
  moderate: "Moderate",
  high: "High",
  critical: "Critical",
};

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function MemoryPressureIndicator({ pressure }: { pressure: MemoryPressure }) {
  const colors = PRESSURE_COLORS[pressure.level];
  const barWidth = Math.min(pressure.percent, 100);

  return (
    <div className="card p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <MemoryStick className="w-5 h-5 text-gray-400" />
          <h3 className="text-sm font-semibold text-gray-900 dark:text-white">Memory Pressure</h3>
        </div>
        <span className={clsx("px-2 py-0.5 text-xs font-semibold rounded-full", colors)}>
          {PRESSURE_LABELS[pressure.level]}
        </span>
      </div>
      <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
        <div
          className={clsx(
            "h-full rounded-full transition-all duration-500",
            pressure.level === "critical"
              ? "bg-error-500"
              : pressure.level === "high"
                ? "bg-orange-500"
                : pressure.level === "moderate"
                  ? "bg-warning-500"
                  : "bg-success-500",
          )}
          style={{ width: `${barWidth}%` }}
        />
      </div>
      <div className="flex justify-between mt-1 text-xs text-gray-500">
        <span>{formatBytes(pressure.totalUsed)} used</span>
        <span>{formatBytes(pressure.totalLimit)} limit</span>
      </div>
      {pressure.containersAtRisk.length > 0 && (
        <div className="mt-2 flex items-center gap-1 text-xs text-warning-500">
          <AlertTriangle className="w-3 h-3" />
          <span>
            {pressure.containersAtRisk.length} container
            {pressure.containersAtRisk.length > 1 ? "s" : ""} above 80% memory
          </span>
        </div>
      )}
    </div>
  );
}

function ContainerDrillDown({ containerId }: { containerId: string }) {
  const history = useMetricsStore((s) => s.getContainerHistory(containerId));
  const latest = useMetricsStore((s) => s.latestContainerMetrics[containerId]);

  if (!latest) return null;

  return (
    <div className="mt-4 space-y-4 pl-6 border-l-2 border-primary-500/30">
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div className="text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {latest.cpu.toFixed(1)}%
          </p>
          <p className="text-xs text-gray-500">CPU</p>
        </div>
        <div className="text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatBytesShort(latest.memory)}
          </p>
          <p className="text-xs text-gray-500">Memory</p>
        </div>
        <div className="text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatBytesShort(latest.networkRx)}/s
          </p>
          <p className="text-xs text-gray-500">Net RX</p>
        </div>
        <div className="text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatBytesShort(latest.networkTx)}/s
          </p>
          <p className="text-xs text-gray-500">Net TX</p>
        </div>
      </div>
      {history.length > 2 && (
        <div className="h-32">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={history}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.4} />
              <XAxis
                dataKey="time"
                stroke="#9CA3AF"
                tick={{ fontSize: 9 }}
                interval="preserveEnd"
              />
              <YAxis stroke="#9CA3AF" tick={{ fontSize: 9 }} />
              <Tooltip contentStyle={CHART_TOOLTIP_STYLE} />
              <Line
                type="monotone"
                dataKey="cpu"
                stroke="#3B82F6"
                strokeWidth={1.5}
                dot={false}
                name="CPU %"
              />
              <Line
                type="monotone"
                dataKey="memoryPercent"
                stroke="#8B5CF6"
                strokeWidth={1.5}
                dot={false}
                name="Mem %"
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Dashboard Component
// ---------------------------------------------------------------------------

export default function Dashboard() {
  const {
    systemInfo,
    performanceMetrics,
    fetchSystemInfo,
    fetchPerformanceMetrics,
    daemonConnected,
  } = useSystemStore();
  const {
    containers,
    stats: containerStats,
    fetchContainers,
    fetchAllRunningStats,
  } = useContainerStore();
  const { projects, fetchProjects } = useProjectStore();
  const { systemHistory, memoryPressure, startStreaming, stopStreaming, latestContainerMetrics } =
    useMetricsStore();

  const [refreshing, setRefreshing] = useState(false);
  const [expandedContainer, setExpandedContainer] = useState<string | null>(null);
  const [chartTab, setChartTab] = useState<"cpu" | "memory" | "network" | "io">("cpu");

  // Initial fetch
  useEffect(() => {
    fetchSystemInfo();
    fetchPerformanceMetrics();
    fetchContainers();
    fetchProjects();
  }, [fetchSystemInfo, fetchPerformanceMetrics, fetchContainers, fetchProjects]);

  // Start real-time metrics streaming
  useEffect(() => {
    if (daemonConnected) {
      startStreaming();
    }
    return () => stopStreaming();
  }, [daemonConnected, startStreaming, stopStreaming]);

  // Periodic supplementary refresh
  useEffect(() => {
    const interval = setInterval(() => {
      if (daemonConnected) {
        fetchPerformanceMetrics();
        fetchAllRunningStats();
      }
    }, 10_000);
    return () => clearInterval(interval);
  }, [daemonConnected, fetchPerformanceMetrics, fetchAllRunningStats]);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await Promise.all([
      fetchSystemInfo(),
      fetchPerformanceMetrics(),
      fetchContainers(),
      fetchAllRunningStats(),
    ]);
    setRefreshing(false);
  }, [fetchSystemInfo, fetchPerformanceMetrics, fetchContainers, fetchAllRunningStats]);

  // -------------------------------------------------------------------------
  // Derived data
  // -------------------------------------------------------------------------

  const runningContainers = useMemo(
    () => containers.filter((c) => c.status === "running"),
    [containers],
  );

  const stats = useMemo(
    () => [
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
    ],
    [systemInfo, projects, containers],
  );

  // Project-level resource aggregates
  const projectAggregates = useMemo(() => {
    const agg: Record<string, { name: string; cpu: number; memory: number; containers: number }> =
      {};
    for (const c of runningContainers) {
      const project = projects.find((p) => p.id === c.projectId);
      const pName = project?.name ?? "Unassigned";
      const key = project?.id ?? "__none__";
      if (!agg[key]) agg[key] = { name: pName, cpu: 0, memory: 0, containers: 0 };
      const m = latestContainerMetrics[c.id];
      if (m) {
        agg[key].cpu += m.cpu;
        agg[key].memory += m.memory;
      }
      agg[key].containers += 1;
    }
    return Object.values(agg).sort((a, b) => b.cpu - a.cpu);
  }, [runningContainers, projects, latestContainerMetrics]);

  const chartTabs = [
    { key: "cpu" as const, label: "CPU", icon: Cpu },
    { key: "memory" as const, label: "Memory", icon: MemoryStick },
    { key: "network" as const, label: "Network", icon: Network },
    { key: "io" as const, label: "I/O", icon: HardDrive },
  ];

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Real-time overview of your container environment
          </p>
        </div>
        <button
          onClick={handleRefresh}
          disabled={refreshing}
          className="btn btn-secondary flex items-center gap-2"
          title="Refresh all"
        >
          <RefreshCw className={clsx("w-4 h-4", refreshing && "animate-spin")} />
          Refresh
        </button>
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

      {/* Memory Pressure + Performance Metrics Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <MemoryPressureIndicator pressure={memoryPressure} />

        {/* Performance Highlight */}
        <div className="card p-4">
          <div className="flex items-center gap-2 mb-3">
            <Gauge className="w-5 h-5 text-primary-500" />
            <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
              Performance Metrics
            </h3>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div className="flex items-center gap-3">
              <Timer className="w-8 h-8 text-success-500" />
              <div>
                <p className="text-xl font-bold text-gray-900 dark:text-white">
                  {performanceMetrics ? `${Math.round(performanceMetrics.warmStartAvgMs)}ms` : "—"}
                </p>
                <p className="text-xs text-gray-500">Warm Start</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <Zap className="w-8 h-8 text-warning-500" />
              <div>
                <p className="text-xl font-bold text-gray-900 dark:text-white">
                  {performanceMetrics ? `${performanceMetrics.speedupFactor.toFixed(1)}x` : "—"}
                </p>
                <p className="text-xs text-gray-500">vs Docker</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <Box className="w-8 h-8 text-primary-500" />
              <div>
                <p className="text-xl font-bold text-gray-900 dark:text-white">
                  {performanceMetrics ? `${Math.round(performanceMetrics.coldStartAvgMs)}ms` : "—"}
                </p>
                <p className="text-xs text-gray-500">Cold Start</p>
              </div>
            </div>
          </div>
          {performanceMetrics && (
            <div className="grid grid-cols-4 gap-2 mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
              <div className="text-center">
                <p className="text-sm font-semibold text-gray-900 dark:text-white">
                  {performanceMetrics.checkpointsActive}
                </p>
                <p className="text-[10px] text-gray-500">Checkpoints</p>
              </div>
              <div className="text-center">
                <p className="text-sm font-semibold text-gray-900 dark:text-white">
                  {performanceMetrics.containersPrewarmed}
                </p>
                <p className="text-[10px] text-gray-500">Prewarmed</p>
              </div>
              <div className="text-center">
                <p className="text-sm font-semibold text-gray-900 dark:text-white">
                  {(performanceMetrics.lazyLoadHitRate * 100).toFixed(0)}%
                </p>
                <p className="text-[10px] text-gray-500">Lazy Hit</p>
              </div>
              <div className="text-center">
                <p className="text-sm font-semibold text-gray-900 dark:text-white">
                  {(performanceMetrics.prewarmHitRate * 100).toFixed(0)}%
                </p>
                <p className="text-[10px] text-gray-500">Prewarm Hit</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Real-Time System Charts */}
      <div className="card p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Activity className="w-5 h-5 text-primary-500" />
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
              Real-Time System Metrics
            </h2>
          </div>
          <div className="flex rounded-lg overflow-hidden border border-gray-200 dark:border-gray-700">
            {chartTabs.map((tab) => (
              <button
                key={tab.key}
                onClick={() => setChartTab(tab.key)}
                className={clsx(
                  "flex items-center gap-1 px-3 py-1.5 text-xs font-medium transition-colors",
                  chartTab === tab.key
                    ? "bg-primary-500 text-white"
                    : "text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700",
                )}
              >
                <tab.icon className="w-3 h-3" />
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            {chartTab === "cpu" ? (
              <AreaChart data={systemHistory}>
                <defs>
                  <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#3B82F6" stopOpacity={0.4} />
                    <stop offset="95%" stopColor="#3B82F6" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.4} />
                <XAxis
                  dataKey="time"
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  interval="preserveEnd"
                />
                <YAxis stroke="#9CA3AF" tick={{ fontSize: 10 }} domain={[0, 100]} unit="%" />
                <Tooltip contentStyle={CHART_TOOLTIP_STYLE} />
                <Area
                  type="monotone"
                  dataKey="cpu"
                  stroke="#3B82F6"
                  fill="url(#cpuGrad)"
                  strokeWidth={2}
                  name="CPU %"
                  dot={false}
                  isAnimationActive={false}
                />
              </AreaChart>
            ) : chartTab === "memory" ? (
              <AreaChart data={systemHistory}>
                <defs>
                  <linearGradient id="memGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#8B5CF6" stopOpacity={0.4} />
                    <stop offset="95%" stopColor="#8B5CF6" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.4} />
                <XAxis
                  dataKey="time"
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  interval="preserveEnd"
                />
                <YAxis
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  tickFormatter={(v: number) => formatBytesShort(v)}
                />
                <Tooltip
                  contentStyle={CHART_TOOLTIP_STYLE}
                  formatter={(v: number) => [formatBytes(v), "Memory"]}
                />
                <Area
                  type="monotone"
                  dataKey="memory"
                  stroke="#8B5CF6"
                  fill="url(#memGrad)"
                  strokeWidth={2}
                  name="Memory"
                  dot={false}
                  isAnimationActive={false}
                />
              </AreaChart>
            ) : chartTab === "network" ? (
              <LineChart data={systemHistory}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.4} />
                <XAxis
                  dataKey="time"
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  interval="preserveEnd"
                />
                <YAxis
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  tickFormatter={(v: number) => formatBytesShort(v)}
                />
                <Tooltip
                  contentStyle={CHART_TOOLTIP_STYLE}
                  formatter={(v: number, name: string) => [formatBytes(v), name]}
                />
                <Legend />
                <Line
                  type="monotone"
                  dataKey="networkRx"
                  stroke="#10B981"
                  strokeWidth={2}
                  name="RX"
                  dot={false}
                  isAnimationActive={false}
                />
                <Line
                  type="monotone"
                  dataKey="networkTx"
                  stroke="#F59E0B"
                  strokeWidth={2}
                  name="TX"
                  dot={false}
                  isAnimationActive={false}
                />
              </LineChart>
            ) : (
              <LineChart data={systemHistory}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.4} />
                <XAxis
                  dataKey="time"
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  interval="preserveEnd"
                />
                <YAxis
                  stroke="#9CA3AF"
                  tick={{ fontSize: 10 }}
                  tickFormatter={(v: number) => formatBytesShort(v)}
                />
                <Tooltip
                  contentStyle={CHART_TOOLTIP_STYLE}
                  formatter={(v: number, name: string) => [formatBytes(v), name]}
                />
                <Legend />
                <Line
                  type="monotone"
                  dataKey="blockRead"
                  stroke="#06B6D4"
                  strokeWidth={2}
                  name="Read"
                  dot={false}
                  isAnimationActive={false}
                />
                <Line
                  type="monotone"
                  dataKey="blockWrite"
                  stroke="#EC4899"
                  strokeWidth={2}
                  name="Write"
                  dot={false}
                  isAnimationActive={false}
                />
              </LineChart>
            )}
          </ResponsiveContainer>
        </div>
      </div>

      {/* Project-Level Aggregates */}
      {projectAggregates.length > 0 && (
        <div className="card">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
              Project Resource Usage
            </h2>
          </div>
          <div className="divide-y divide-gray-200 dark:divide-gray-700">
            {projectAggregates.map((pa) => (
              <div key={pa.name} className="px-6 py-3 flex items-center gap-4">
                <FolderKanban className="w-5 h-5 text-primary-500" />
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-gray-900 dark:text-white truncate">{pa.name}</p>
                  <p className="text-xs text-gray-500">
                    {pa.containers} container{pa.containers > 1 ? "s" : ""}
                  </p>
                </div>
                <div className="flex items-center gap-4 text-xs text-gray-500">
                  <span className="flex items-center gap-1">
                    <Cpu className="w-3 h-3" /> {pa.cpu.toFixed(1)}%
                  </span>
                  <span className="flex items-center gap-1">
                    <MemoryStick className="w-3 h-3" /> {formatBytes(pa.memory)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Running Containers (expandable drill-down) */}
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
            {runningContainers.map((container) => {
              const cStats = containerStats[container.id];
              const isExpanded = expandedContainer === container.id;

              return (
                <div key={container.id}>
                  <button
                    onClick={() => setExpandedContainer(isExpanded ? null : container.id)}
                    className="w-full container-item hover:bg-gray-50 dark:hover:bg-gray-800/60 transition-colors"
                  >
                    <div className="status-dot status-dot-running" />
                    <div className="flex-1 min-w-0 text-left">
                      <p className="font-medium text-gray-900 dark:text-white truncate">
                        {container.name || container.id.slice(0, 12)}
                      </p>
                      <p className="text-sm text-gray-500 truncate">{container.image}</p>
                    </div>
                    {cStats && (
                      <div className="hidden sm:flex items-center gap-4 text-xs text-gray-500">
                        <div className="flex items-center gap-1" title="CPU">
                          <Cpu className="w-3 h-3" />
                          <span>{cStats.cpuPercent.toFixed(1)}%</span>
                        </div>
                        <div className="flex items-center gap-1" title="Memory">
                          <HardDrive className="w-3 h-3" />
                          <span>{formatBytes(cStats.memoryUsage)}</span>
                        </div>
                      </div>
                    )}
                    {container.hasCheckpoint && (
                      <span className="badge badge-running">
                        <Zap className="w-3 h-3 mr-1" />
                        Warm
                      </span>
                    )}
                    {isExpanded ? (
                      <ChevronUp className="w-4 h-4 text-gray-400" />
                    ) : (
                      <ChevronDown className="w-4 h-4 text-gray-400" />
                    )}
                  </button>
                  {isExpanded && <ContainerDrillDown containerId={container.id} />}
                </div>
              );
            })}
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
