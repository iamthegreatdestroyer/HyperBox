import { useState, useCallback } from "react";
import { X, Plus, Trash2, Server, Terminal, Network, HardDrive, Variable } from "lucide-react";
import { CreateContainerRequest, PortMapping, useContainerStore } from "../stores";
import { clsx } from "clsx";

interface CreateContainerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export default function CreateContainerModal({
  isOpen,
  onClose,
  onSuccess,
}: CreateContainerModalProps) {
  const { createContainer, loading, error, clearError } = useContainerStore();

  // Form state
  const [image, setImage] = useState("");
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [envVars, setEnvVars] = useState<{ key: string; value: string }[]>([]);
  const [ports, setPorts] = useState<{ host: string; container: string; protocol: string }[]>([]);
  const [volumes, setVolumes] = useState<{ host: string; container: string }[]>([]);

  // Active tab for organized form
  const [activeTab, setActiveTab] = useState<"basic" | "env" | "ports" | "volumes">("basic");

  const resetForm = useCallback(() => {
    setImage("");
    setName("");
    setCommand("");
    setEnvVars([]);
    setPorts([]);
    setVolumes([]);
    setActiveTab("basic");
    clearError();
  }, [clearError]);

  const handleClose = useCallback(() => {
    resetForm();
    onClose();
  }, [resetForm, onClose]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!image.trim()) {
      return;
    }

    const request: CreateContainerRequest = {
      image: image.trim(),
      name: name.trim() || undefined,
      command: command.trim() ? command.trim().split(/\s+/) : undefined,
      env: envVars
        .filter((e) => e.key.trim())
        .map((e) => `${e.key.trim()}=${e.value}`),
      ports: ports
        .filter((p) => p.host && p.container)
        .map(
          (p): PortMapping => ({
            host: parseInt(p.host, 10),
            container: parseInt(p.container, 10),
            protocol: p.protocol || "tcp",
          })
        ),
      volumes: volumes
        .filter((v) => v.host.trim() && v.container.trim())
        .map((v) => `${v.host.trim()}:${v.container.trim()}`),
    };

    const result = await createContainer(request);
    if (result) {
      handleClose();
      onSuccess?.();
    }
  };

  // Environment variable helpers
  const addEnvVar = () => setEnvVars([...envVars, { key: "", value: "" }]);
  const removeEnvVar = (index: number) =>
    setEnvVars(envVars.filter((_, i) => i !== index));
  const updateEnvVar = (
    index: number,
    field: "key" | "value",
    value: string
  ) => {
    const updated = [...envVars];
    updated[index][field] = value;
    setEnvVars(updated);
  };

  // Port mapping helpers
  const addPort = () =>
    setPorts([...ports, { host: "", container: "", protocol: "tcp" }]);
  const removePort = (index: number) =>
    setPorts(ports.filter((_, i) => i !== index));
  const updatePort = (
    index: number,
    field: "host" | "container" | "protocol",
    value: string
  ) => {
    const updated = [...ports];
    updated[index][field] = value;
    setPorts(updated);
  };

  // Volume helpers
  const addVolume = () => setVolumes([...volumes, { host: "", container: "" }]);
  const removeVolume = (index: number) =>
    setVolumes(volumes.filter((_, i) => i !== index));
  const updateVolume = (
    index: number,
    field: "host" | "container",
    value: string
  ) => {
    const updated = [...volumes];
    updated[index][field] = value;
    setVolumes(updated);
  };

  if (!isOpen) return null;

  const tabs = [
    { id: "basic" as const, label: "Basic", icon: Server },
    { id: "env" as const, label: "Environment", icon: Variable },
    { id: "ports" as const, label: "Ports", icon: Network },
    { id: "volumes" as const, label: "Volumes", icon: HardDrive },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={handleClose}
      />

      {/* Modal */}
      <div className="relative w-full max-w-2xl max-h-[90vh] overflow-hidden rounded-xl bg-white dark:bg-gray-800 shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Create Container
          </h2>
          <button
            onClick={handleClose}
            className="p-2 rounded-lg text-gray-500 hover:text-gray-700 hover:bg-gray-100 dark:hover:text-gray-300 dark:hover:bg-gray-700 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Error Display */}
        {error && (
          <div className="mx-6 mt-4 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
            <p className="text-sm text-red-700 dark:text-red-400">{error}</p>
          </div>
        )}

        {/* Tab Navigation */}
        <div className="flex border-b border-gray-200 dark:border-gray-700 px-6">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                "flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors",
                activeTab === tab.id
                  ? "border-primary-500 text-primary-600 dark:text-primary-400"
                  : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
              )}
            >
              <tab.icon className="w-4 h-4" />
              {tab.label}
              {tab.id === "env" && envVars.length > 0 && (
                <span className="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-primary-100 dark:bg-primary-900 text-primary-600 dark:text-primary-400">
                  {envVars.length}
                </span>
              )}
              {tab.id === "ports" && ports.length > 0 && (
                <span className="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-primary-100 dark:bg-primary-900 text-primary-600 dark:text-primary-400">
                  {ports.length}
                </span>
              )}
              {tab.id === "volumes" && volumes.length > 0 && (
                <span className="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-primary-100 dark:bg-primary-900 text-primary-600 dark:text-primary-400">
                  {volumes.length}
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit}>
          <div className="p-6 overflow-y-auto max-h-[50vh]">
            {/* Basic Tab */}
            {activeTab === "basic" && (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Image <span className="text-red-500">*</span>
                  </label>
                  <input
                    type="text"
                    value={image}
                    onChange={(e) => setImage(e.target.value)}
                    placeholder="e.g., nginx:latest, ubuntu:22.04"
                    className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                    required
                  />
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    Docker image name with optional tag
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Container Name
                  </label>
                  <input
                    type="text"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g., my-web-server (optional)"
                    className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                  />
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    Leave empty for auto-generated name
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    <div className="flex items-center gap-2">
                      <Terminal className="w-4 h-4" />
                      Command
                    </div>
                  </label>
                  <input
                    type="text"
                    value={command}
                    onChange={(e) => setCommand(e.target.value)}
                    placeholder="e.g., /bin/bash -c 'echo hello'"
                    className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
                  />
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    Override the default command (optional)
                  </p>
                </div>
              </div>
            )}

            {/* Environment Variables Tab */}
            {activeTab === "env" && (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Define environment variables for the container
                  </p>
                  <button
                    type="button"
                    onClick={addEnvVar}
                    className="flex items-center gap-1 px-3 py-1.5 text-sm rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 hover:bg-primary-100 dark:hover:bg-primary-900/50 transition-colors"
                  >
                    <Plus className="w-4 h-4" />
                    Add Variable
                  </button>
                </div>

                {envVars.length === 0 ? (
                  <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                    <Variable className="w-12 h-12 mx-auto mb-2 opacity-50" />
                    <p>No environment variables defined</p>
                    <p className="text-xs mt-1">Click "Add Variable" to add one</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {envVars.map((env, index) => (
                      <div key={index} className="flex items-center gap-2">
                        <input
                          type="text"
                          value={env.key}
                          onChange={(e) =>
                            updateEnvVar(index, "key", e.target.value)
                          }
                          placeholder="KEY"
                          className="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
                        />
                        <span className="text-gray-400">=</span>
                        <input
                          type="text"
                          value={env.value}
                          onChange={(e) =>
                            updateEnvVar(index, "value", e.target.value)
                          }
                          placeholder="value"
                          className="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
                        />
                        <button
                          type="button"
                          onClick={() => removeEnvVar(index)}
                          className="p-2 rounded-lg text-red-500 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Ports Tab */}
            {activeTab === "ports" && (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Map container ports to host ports
                  </p>
                  <button
                    type="button"
                    onClick={addPort}
                    className="flex items-center gap-1 px-3 py-1.5 text-sm rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 hover:bg-primary-100 dark:hover:bg-primary-900/50 transition-colors"
                  >
                    <Plus className="w-4 h-4" />
                    Add Port
                  </button>
                </div>

                {ports.length === 0 ? (
                  <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                    <Network className="w-12 h-12 mx-auto mb-2 opacity-50" />
                    <p>No port mappings defined</p>
                    <p className="text-xs mt-1">Click "Add Port" to expose a port</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <div className="grid grid-cols-[1fr_1fr_100px_40px] gap-2 text-xs font-medium text-gray-500 dark:text-gray-400 px-1">
                      <span>Host Port</span>
                      <span>Container Port</span>
                      <span>Protocol</span>
                      <span></span>
                    </div>
                    {ports.map((port, index) => (
                      <div
                        key={index}
                        className="grid grid-cols-[1fr_1fr_100px_40px] gap-2 items-center"
                      >
                        <input
                          type="number"
                          value={port.host}
                          onChange={(e) =>
                            updatePort(index, "host", e.target.value)
                          }
                          placeholder="8080"
                          min="1"
                          max="65535"
                          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                        />
                        <input
                          type="number"
                          value={port.container}
                          onChange={(e) =>
                            updatePort(index, "container", e.target.value)
                          }
                          placeholder="80"
                          min="1"
                          max="65535"
                          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                        />
                        <select
                          value={port.protocol}
                          onChange={(e) =>
                            updatePort(index, "protocol", e.target.value)
                          }
                          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                        >
                          <option value="tcp">TCP</option>
                          <option value="udp">UDP</option>
                        </select>
                        <button
                          type="button"
                          onClick={() => removePort(index)}
                          className="p-2 rounded-lg text-red-500 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Volumes Tab */}
            {activeTab === "volumes" && (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Mount host directories or volumes into the container
                  </p>
                  <button
                    type="button"
                    onClick={addVolume}
                    className="flex items-center gap-1 px-3 py-1.5 text-sm rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 hover:bg-primary-100 dark:hover:bg-primary-900/50 transition-colors"
                  >
                    <Plus className="w-4 h-4" />
                    Add Volume
                  </button>
                </div>

                {volumes.length === 0 ? (
                  <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                    <HardDrive className="w-12 h-12 mx-auto mb-2 opacity-50" />
                    <p>No volume mounts defined</p>
                    <p className="text-xs mt-1">Click "Add Volume" to mount a volume</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <div className="grid grid-cols-[1fr_1fr_40px] gap-2 text-xs font-medium text-gray-500 dark:text-gray-400 px-1">
                      <span>Host Path</span>
                      <span>Container Path</span>
                      <span></span>
                    </div>
                    {volumes.map((volume, index) => (
                      <div
                        key={index}
                        className="grid grid-cols-[1fr_1fr_40px] gap-2 items-center"
                      >
                        <input
                          type="text"
                          value={volume.host}
                          onChange={(e) =>
                            updateVolume(index, "host", e.target.value)
                          }
                          placeholder="/host/path or volume_name"
                          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
                        />
                        <input
                          type="text"
                          value={volume.container}
                          onChange={(e) =>
                            updateVolume(index, "container", e.target.value)
                          }
                          placeholder="/container/path"
                          className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
                        />
                        <button
                          type="button"
                          onClick={() => removeVolume(index)}
                          className="p-2 rounded-lg text-red-500 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
            <button
              type="button"
              onClick={handleClose}
              className="px-4 py-2 text-sm font-medium rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !image.trim()}
              className={clsx(
                "flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg transition-colors",
                loading || !image.trim()
                  ? "bg-gray-300 dark:bg-gray-600 text-gray-500 dark:text-gray-400 cursor-not-allowed"
                  : "bg-primary-600 hover:bg-primary-700 text-white"
              )}
            >
              {loading ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <Plus className="w-4 h-4" />
                  Create Container
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
