import { useState } from "react";
import { Moon, Sun, Monitor, Folder, RefreshCw, Trash2 } from "lucide-react";
import { useSystemStore } from "../stores/system";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { clsx } from "clsx";
import { UpdateSettings as UpdateSettingsComponent } from "../components/UpdateChecker";

export default function Settings() {
  const { theme, setTheme, startDaemon, stopDaemon, daemonConnected, loading } = useSystemStore();
  const [settings, setSettings] = useState({
    dataDir: "",
    defaultRuntime: "crun",
    enablePrewarm: true,
    enableCriu: true,
    enableLazyLoading: true,
    checkpointTtlHours: 24,
    maxConcurrentPulls: 3,
    portRangeStart: 32768,
    portRangeEnd: 60999,
  });
  const [pruning, setPruning] = useState(false);

  const handleSelectDataDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Data Directory",
    });
    if (selected) {
      setSettings((s) => ({ ...s, dataDir: selected as string }));
    }
  };

  const handlePruneSystem = async () => {
    setPruning(true);
    try {
      await invoke("prune_system", { all: true });
    } catch (error) {
      console.error("Failed to prune:", error);
    }
    setPruning(false);
  };

  const themeOptions = [
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
    { value: "system", label: "System", icon: Monitor },
  ] as const;

  const runtimeOptions = [
    { value: "crun", label: "crun (47ms - Recommended)" },
    { value: "youki", label: "youki (Rust native)" },
    { value: "runc", label: "runc (Docker default)" },
  ];

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Settings</h1>
        <p className="text-gray-500 dark:text-gray-400">
          Configure HyperBox preferences and optimizations
        </p>
      </div>

      {/* Appearance */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Appearance</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Theme
            </label>
            <div className="flex gap-2">
              {themeOptions.map((option) => (
                <button
                  key={option.value}
                  onClick={() => setTheme(option.value)}
                  className={clsx(
                    "flex items-center gap-2 px-4 py-2 rounded-lg border transition-colors",
                    theme === option.value
                      ? "bg-primary-500 text-white border-primary-500"
                      : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:border-primary-400",
                  )}
                >
                  <option.icon className="w-4 h-4" />
                  {option.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Daemon */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Daemon</h2>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">Daemon Status</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {daemonConnected ? "Running and connected" : "Not running"}
              </p>
            </div>
            <button
              onClick={daemonConnected ? stopDaemon : startDaemon}
              disabled={loading}
              className={clsx("btn", daemonConnected ? "btn-danger" : "btn-primary")}
            >
              {loading ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : daemonConnected ? (
                "Stop Daemon"
              ) : (
                "Start Daemon"
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Runtime */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Runtime</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Default Container Runtime
            </label>
            <select
              value={settings.defaultRuntime}
              onChange={(e) => setSettings((s) => ({ ...s, defaultRuntime: e.target.value }))}
              className="input w-full max-w-md"
            >
              {runtimeOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Data Directory
            </label>
            <div className="flex gap-2 max-w-md">
              <input
                type="text"
                value={settings.dataDir}
                readOnly
                placeholder="Default: ~/.hyperbox"
                className="input flex-1"
              />
              <button onClick={handleSelectDataDir} className="btn btn-secondary">
                <Folder className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Performance Optimizations */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Performance Optimizations
        </h2>
        <div className="space-y-4">
          <label className="flex items-center justify-between">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">CRIU Checkpointing</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Enable warm starts with process checkpoint/restore
              </p>
            </div>
            <input
              type="checkbox"
              checked={settings.enableCriu}
              onChange={(e) => setSettings((s) => ({ ...s, enableCriu: e.target.checked }))}
              className="toggle"
            />
          </label>

          <label className="flex items-center justify-between">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">Pre-warming</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Predictively start containers based on usage patterns
              </p>
            </div>
            <input
              type="checkbox"
              checked={settings.enablePrewarm}
              onChange={(e) => setSettings((s) => ({ ...s, enablePrewarm: e.target.checked }))}
              className="toggle"
            />
          </label>

          <label className="flex items-center justify-between">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">Lazy Layer Loading</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Use eStargz format for on-demand image layers
              </p>
            </div>
            <input
              type="checkbox"
              checked={settings.enableLazyLoading}
              onChange={(e) => setSettings((s) => ({ ...s, enableLazyLoading: e.target.checked }))}
              className="toggle"
            />
          </label>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Checkpoint TTL (hours)
            </label>
            <input
              type="number"
              value={settings.checkpointTtlHours}
              onChange={(e) =>
                setSettings((s) => ({ ...s, checkpointTtlHours: parseInt(e.target.value) || 24 }))
              }
              className="input w-32"
              min={1}
              max={168}
            />
          </div>
        </div>
      </div>

      {/* Network */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Network</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Port Allocation Range
            </label>
            <div className="flex items-center gap-2 max-w-md">
              <input
                type="number"
                value={settings.portRangeStart}
                onChange={(e) =>
                  setSettings((s) => ({ ...s, portRangeStart: parseInt(e.target.value) || 32768 }))
                }
                className="input w-28"
                min={1024}
                max={65535}
              />
              <span className="text-gray-500">to</span>
              <input
                type="number"
                value={settings.portRangeEnd}
                onChange={(e) =>
                  setSettings((s) => ({ ...s, portRangeEnd: parseInt(e.target.value) || 60999 }))
                }
                className="input w-28"
                min={1024}
                max={65535}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Cleanup */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Cleanup</h2>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">Prune System</p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Remove all unused containers, images, networks, and build cache
              </p>
            </div>
            <button onClick={handlePruneSystem} disabled={pruning} className="btn btn-danger">
              {pruning ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <>
                  <Trash2 className="w-4 h-4" />
                  Prune All
                </>
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Updates */}
      <UpdateSettingsComponent />
    </div>
  );
}
