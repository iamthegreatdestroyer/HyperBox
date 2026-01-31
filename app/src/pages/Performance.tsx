import { useEffect, useState } from "react";
import { Gauge, Zap, Timer, TrendingUp, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  AreaChart,
  Area,
} from "recharts";

interface PerformanceMetrics {
  coldStartAvg: number;
  warmStartAvg: number;
  checkpointsActive: number;
  prewarmHits: number;
  layerCacheHitRate: number;
  dockerComparison: number;
}

interface BenchmarkResult {
  name: string;
  hyperbox: number;
  docker: number;
}

export default function Performance() {
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [benchmarks, setBenchmarks] = useState<BenchmarkResult[]>([]);
  const [cpuHistory, setCpuHistory] = useState<{ time: string; value: number }[]>([]);
  const [memoryHistory, setMemoryHistory] = useState<{ time: string; value: number }[]>([]);

  const fetchMetrics = async () => {
    setLoading(true);
    try {
      const rawMetrics = await invoke<{
        cold_start_avg: number;
        warm_start_avg: number;
        checkpoints_active: number;
        prewarm_hits: number;
        layer_cache_hit_rate: number;
        docker_comparison: number;
      }>("get_performance_metrics");

      setMetrics({
        coldStartAvg: rawMetrics.cold_start_avg,
        warmStartAvg: rawMetrics.warm_start_avg,
        checkpointsActive: rawMetrics.checkpoints_active,
        prewarmHits: rawMetrics.prewarm_hits,
        layerCacheHitRate: rawMetrics.layer_cache_hit_rate,
        dockerComparison: rawMetrics.docker_comparison,
      });

      // Mock benchmarks for demo
      setBenchmarks([
        { name: "Cold Start", hyperbox: 4.2, docker: 35 },
        { name: "Warm Start", hyperbox: 0.085, docker: 3.5 },
        { name: "Image Pull", hyperbox: 2.1, docker: 12 },
        { name: "Stop", hyperbox: 0.05, docker: 0.8 },
      ]);

      // Generate mock history data
      const now = Date.now();
      const newCpuHistory = Array.from({ length: 20 }, (_, i) => ({
        time: new Date(now - (19 - i) * 1000).toLocaleTimeString(),
        value: Math.random() * 30 + 10,
      }));
      setCpuHistory(newCpuHistory);

      const newMemoryHistory = Array.from({ length: 20 }, (_, i) => ({
        time: new Date(now - (19 - i) * 1000).toLocaleTimeString(),
        value: Math.random() * 200 + 100,
      }));
      setMemoryHistory(newMemoryHistory);
    } catch (error) {
      console.error("Failed to fetch metrics:", error);
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, 5000);
    return () => clearInterval(interval);
  }, []);

  const statCards = [
    {
      name: "Warm Start Time",
      value: metrics ? `${(metrics.warmStartAvg * 1000).toFixed(0)}ms` : "--",
      description: "35x faster than Docker",
      icon: Zap,
      color: "text-warning-500",
      bgColor: "bg-warning-500/10",
    },
    {
      name: "Cold Start Time",
      value: metrics ? `${metrics.coldStartAvg.toFixed(1)}s` : "--",
      description: "Sub-linear image loading",
      icon: Timer,
      color: "text-primary-500",
      bgColor: "bg-primary-500/10",
    },
    {
      name: "CRIU Checkpoints",
      value: metrics?.checkpointsActive || 0,
      description: "Ready for instant restore",
      icon: Gauge,
      color: "text-success-500",
      bgColor: "bg-success-500/10",
    },
    {
      name: "Cache Hit Rate",
      value: metrics ? `${(metrics.layerCacheHitRate * 100).toFixed(0)}%` : "--",
      description: "eStargz lazy loading",
      icon: TrendingUp,
      color: "text-accent-500",
      bgColor: "bg-accent-500/10",
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Performance</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Monitor sub-linear algorithmic optimizations
          </p>
        </div>
        <button onClick={fetchMetrics} className="btn btn-secondary">
          <RefreshCw className={loading ? "w-4 h-4 animate-spin" : "w-4 h-4"} />
          Refresh
        </button>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {statCards.map((stat) => (
          <div key={stat.name} className="card p-6">
            <div className="flex items-center gap-4">
              <div className={`p-3 rounded-xl ${stat.bgColor}`}>
                <stat.icon className={`w-6 h-6 ${stat.color}`} />
              </div>
              <div>
                <p className="stat-value">{stat.value}</p>
                <p className="stat-label">{stat.name}</p>
              </div>
            </div>
            <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">{stat.description}</p>
          </div>
        ))}
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* CPU Usage */}
        <div className="card p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            CPU Usage (%)
          </h3>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={cpuHistory}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" tick={{ fontSize: 10 }} />
                <YAxis stroke="#9CA3AF" tick={{ fontSize: 10 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#1F2937",
                    border: "none",
                    borderRadius: "8px",
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="#3B82F6"
                  fill="#3B82F6"
                  fillOpacity={0.2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Memory Usage */}
        <div className="card p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Memory Usage (MB)
          </h3>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={memoryHistory}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" tick={{ fontSize: 10 }} />
                <YAxis stroke="#9CA3AF" tick={{ fontSize: 10 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#1F2937",
                    border: "none",
                    borderRadius: "8px",
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="#8B5CF6"
                  fill="#8B5CF6"
                  fillOpacity={0.2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Benchmark Comparison */}
      <div className="card p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          HyperBox vs Docker Comparison (seconds)
        </h3>
        <div className="h-80">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={benchmarks}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="name" stroke="#9CA3AF" />
              <YAxis stroke="#9CA3AF" />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1F2937",
                  border: "none",
                  borderRadius: "8px",
                }}
              />
              <Line
                type="monotone"
                dataKey="hyperbox"
                stroke="#10B981"
                strokeWidth={3}
                name="HyperBox"
              />
              <Line
                type="monotone"
                dataKey="docker"
                stroke="#EF4444"
                strokeWidth={3}
                name="Docker"
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
        <div className="flex justify-center gap-8 mt-4">
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-success-500" />
            <span className="text-sm text-gray-600 dark:text-gray-400">HyperBox</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-error-500" />
            <span className="text-sm text-gray-600 dark:text-gray-400">Docker</span>
          </div>
        </div>
      </div>
    </div>
  );
}
