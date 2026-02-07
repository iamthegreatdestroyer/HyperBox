import { useCallback, useEffect, useRef, useState } from "react";
import {
  Plus,
  X,
  Terminal as TerminalIcon,
  Box,
  RefreshCw,
  Maximize2,
  Minimize2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { clsx } from "clsx";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { useContainerStore } from "../stores/containers";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TerminalTab {
  id: string;
  label: string;
  containerId?: string;
  containerName?: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const XTERM_THEME = {
  background: "#0D1117",
  foreground: "#C9D1D9",
  cursor: "#58A6FF",
  cursorAccent: "#0D1117",
  selectionBackground: "#264F78",
  selectionForeground: "#FFFFFF",
  black: "#484F58",
  red: "#FF7B72",
  green: "#3FB950",
  yellow: "#D29922",
  blue: "#58A6FF",
  magenta: "#BC8CFF",
  cyan: "#39D2C0",
  white: "#B1BAC4",
  brightBlack: "#6E7681",
  brightRed: "#FFA198",
  brightGreen: "#56D364",
  brightYellow: "#E3B341",
  brightBlue: "#79C0FF",
  brightMagenta: "#D2A8FF",
  brightCyan: "#56D4DD",
  brightWhite: "#F0F6FC",
};

let tabIdCounter = 0;
function nextTabId(): string {
  tabIdCounter += 1;
  return `tab-${tabIdCounter}`;
}

// ---------------------------------------------------------------------------
// TerminalInstance – manages a single xterm.js + shell session
// ---------------------------------------------------------------------------

interface TerminalInstanceProps {
  tab: TerminalTab;
  active: boolean;
}

function TerminalInstance({ tab, active }: TerminalInstanceProps) {
  const termRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const inputBufferRef = useRef<string>("");
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef<number>(-1);

  // Write a styled prompt
  const writePrompt = useCallback(() => {
    const xt = xtermRef.current;
    if (!xt) return;
    if (tab.containerId) {
      xt.write(
        `\r\n\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
      );
    } else {
      xt.write("\r\n\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
    }
  }, [tab.containerId, tab.containerName]);

  // Execute a command by invoking the Tauri backend
  const executeCommand = useCallback(
    async (cmd: string) => {
      const xt = xtermRef.current;
      if (!xt) return;
      const trimmed = cmd.trim();
      if (!trimmed) {
        writePrompt();
        return;
      }

      // Build-in commands
      if (trimmed === "clear") {
        xt.clear();
        xt.write("\x1b[2J\x1b[H");
        writePrompt();
        return;
      }

      if (trimmed === "help") {
        xt.write("\r\n");
        xt.write("\x1b[1;36mHyperBox Terminal\x1b[0m\r\n\r\n");
        xt.write("  \x1b[33mclear\x1b[0m          Clear terminal\r\n");
        xt.write("  \x1b[33mhelp\x1b[0m           Show this help\r\n");
        xt.write("  \x1b[33mcontainers\x1b[0m     List running containers\r\n");
        xt.write("  \x1b[33mstats <id>\x1b[0m     Show container stats\r\n");
        xt.write("  \x1b[33mstart <id>\x1b[0m     Start a container\r\n");
        xt.write("  \x1b[33mstop <id>\x1b[0m      Stop a container\r\n");
        xt.write("  \x1b[33mrestart <id>\x1b[0m   Restart a container\r\n");
        xt.write("  \x1b[33minfo\x1b[0m           System information\r\n");
        writePrompt();
        return;
      }

      // Save to history
      historyRef.current.push(trimmed);
      historyIndexRef.current = historyRef.current.length;

      xt.write("\r\n");

      try {
        if (trimmed === "containers") {
          const containers =
            await invoke<{ id: string; name: string; image: string; status: string }[]>(
              "list_containers",
            );
          if (containers.length === 0) {
            xt.write("\x1b[33mNo containers found.\x1b[0m");
          } else {
            xt.write(
              `\x1b[1m${"ID".padEnd(14)}${"NAME".padEnd(24)}${"IMAGE".padEnd(30)}STATUS\x1b[0m\r\n`,
            );
            for (const c of containers) {
              const statusColor =
                c.status === "running"
                  ? "\x1b[32m"
                  : c.status === "exited"
                    ? "\x1b[31m"
                    : "\x1b[33m";
              xt.write(
                `${c.id.slice(0, 12).padEnd(14)}${(c.name || "").slice(0, 22).padEnd(24)}${c.image.slice(0, 28).padEnd(30)}${statusColor}${c.status}\x1b[0m\r\n`,
              );
            }
          }
        } else if (trimmed === "info") {
          const info = await invoke<{
            version: string;
            api_version: string;
            runtime: string;
            os: string;
            arch: string;
            containers_running: number;
            containers_paused: number;
            containers_stopped: number;
            images: number;
          }>("get_system_info");
          xt.write(`\x1b[1;36mHyperBox System Info\x1b[0m\r\n`);
          xt.write(`  Version:    ${info.version}\r\n`);
          xt.write(`  API:        ${info.api_version}\r\n`);
          xt.write(`  Runtime:    ${info.runtime}\r\n`);
          xt.write(`  OS/Arch:    ${info.os}/${info.arch}\r\n`);
          xt.write(`  Running:    \x1b[32m${info.containers_running}\x1b[0m\r\n`);
          xt.write(`  Paused:     \x1b[33m${info.containers_paused}\x1b[0m\r\n`);
          xt.write(`  Stopped:    \x1b[31m${info.containers_stopped}\x1b[0m\r\n`);
          xt.write(`  Images:     ${info.images}\r\n`);
        } else if (trimmed.startsWith("stats ")) {
          const id = trimmed.slice(6).trim();
          const stats = await invoke<{
            cpu_percent: number;
            memory_usage: number;
            memory_limit: number;
            memory_percent: number;
            network_rx: number;
            network_tx: number;
            block_read: number;
            block_write: number;
          }>("get_container_stats", { containerId: id });
          xt.write(`\x1b[1;36mStats for ${id.slice(0, 12)}\x1b[0m\r\n`);
          xt.write(`  CPU:     ${stats.cpu_percent.toFixed(1)}%\r\n`);
          xt.write(
            `  Memory:  ${(stats.memory_usage / 1024 / 1024).toFixed(1)} MB / ${(stats.memory_limit / 1024 / 1024).toFixed(0)} MB (${stats.memory_percent.toFixed(1)}%)\r\n`,
          );
          xt.write(`  Net RX:  ${(stats.network_rx / 1024).toFixed(1)} KB\r\n`);
          xt.write(`  Net TX:  ${(stats.network_tx / 1024).toFixed(1)} KB\r\n`);
          xt.write(`  Blk R:   ${(stats.block_read / 1024).toFixed(1)} KB\r\n`);
          xt.write(`  Blk W:   ${(stats.block_write / 1024).toFixed(1)} KB\r\n`);
        } else if (trimmed.startsWith("start ")) {
          const id = trimmed.slice(6).trim();
          await invoke("start_container", { containerId: id });
          xt.write(`\x1b[32mContainer ${id.slice(0, 12)} started.\x1b[0m`);
        } else if (trimmed.startsWith("stop ")) {
          const id = trimmed.slice(5).trim();
          await invoke("stop_container", { containerId: id });
          xt.write(`\x1b[33mContainer ${id.slice(0, 12)} stopped.\x1b[0m`);
        } else if (trimmed.startsWith("restart ")) {
          const id = trimmed.slice(8).trim();
          await invoke("restart_container", { containerId: id });
          xt.write(`\x1b[32mContainer ${id.slice(0, 12)} restarted.\x1b[0m`);
        } else {
          xt.write(`\x1b[31mUnknown command:\x1b[0m ${trimmed}\r\n`);
          xt.write("Type \x1b[33mhelp\x1b[0m for available commands.");
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        xt.write(`\x1b[31mError:\x1b[0m ${msg}`);
      }

      writePrompt();
    },
    [writePrompt],
  );

  // Initialize xterm.js
  useEffect(() => {
    if (!termRef.current || xtermRef.current) return;

    const term = new XTerm({
      theme: XTERM_THEME,
      fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', 'Consolas', monospace",
      fontSize: 13,
      lineHeight: 1.35,
      cursorBlink: true,
      cursorStyle: "bar",
      allowProposedApi: true,
      scrollback: 5000,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(termRef.current);

    // Initial fit
    requestAnimationFrame(() => {
      try {
        fitAddon.fit();
      } catch {
        /* noop if not visible yet */
      }
    });

    // Welcome banner
    term.write("\x1b[1;36m╔══════════════════════════════════════════════╗\x1b[0m\r\n");
    term.write(
      "\x1b[1;36m║\x1b[0m  \x1b[1mHyperBox Terminal\x1b[0m                           \x1b[1;36m║\x1b[0m\r\n",
    );
    term.write(
      "\x1b[1;36m║\x1b[0m  Type \x1b[33mhelp\x1b[0m for available commands             \x1b[1;36m║\x1b[0m\r\n",
    );
    term.write("\x1b[1;36m╚══════════════════════════════════════════════╝\x1b[0m");

    // Write initial prompt
    if (tab.containerId) {
      term.write(
        `\r\n\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
      );
    } else {
      term.write("\r\n\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
    }

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    // Handle input
    term.onData((data) => {
      const code = data.charCodeAt(0);

      // Enter
      if (code === 13) {
        const cmd = inputBufferRef.current;
        inputBufferRef.current = "";
        executeCommand(cmd);
        return;
      }

      // Backspace
      if (code === 127) {
        if (inputBufferRef.current.length > 0) {
          inputBufferRef.current = inputBufferRef.current.slice(0, -1);
          term.write("\b \b");
        }
        return;
      }

      // Escape sequences (arrow keys)
      if (data === "\x1b[A") {
        // Up arrow — history back
        if (historyRef.current.length > 0 && historyIndexRef.current > 0) {
          historyIndexRef.current -= 1;
          const entry = historyRef.current[historyIndexRef.current];
          // Clear current line
          term.write("\r\x1b[K");
          if (tab.containerId) {
            term.write(
              `\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
            );
          } else {
            term.write("\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
          }
          term.write(entry);
          inputBufferRef.current = entry;
        }
        return;
      }

      if (data === "\x1b[B") {
        // Down arrow — history forward
        if (historyIndexRef.current < historyRef.current.length - 1) {
          historyIndexRef.current += 1;
          const entry = historyRef.current[historyIndexRef.current];
          term.write("\r\x1b[K");
          if (tab.containerId) {
            term.write(
              `\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
            );
          } else {
            term.write("\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
          }
          term.write(entry);
          inputBufferRef.current = entry;
        } else {
          historyIndexRef.current = historyRef.current.length;
          term.write("\r\x1b[K");
          if (tab.containerId) {
            term.write(
              `\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
            );
          } else {
            term.write("\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
          }
          inputBufferRef.current = "";
        }
        return;
      }

      // Ignore other escape sequences
      if (data.startsWith("\x1b")) return;

      // Ctrl-C
      if (code === 3) {
        inputBufferRef.current = "";
        term.write("^C");
        if (tab.containerId) {
          term.write(
            `\r\n\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
          );
        } else {
          term.write("\r\n\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
        }
        return;
      }

      // Ctrl-L (clear)
      if (code === 12) {
        term.clear();
        term.write("\x1b[2J\x1b[H");
        if (tab.containerId) {
          term.write(
            `\x1b[36m${tab.containerName || tab.containerId.slice(0, 12)}\x1b[0m:\x1b[33m~\x1b[0m$ `,
          );
        } else {
          term.write("\x1b[32mhyperbox\x1b[0m:\x1b[33m~\x1b[0m$ ");
        }
        inputBufferRef.current = "";
        return;
      }

      // Printable characters
      if (code >= 32) {
        inputBufferRef.current += data;
        term.write(data);
      }
    });

    return () => {
      term.dispose();
      xtermRef.current = null;
      fitAddonRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Re-fit when tab becomes active or window resizes
  useEffect(() => {
    if (!active || !fitAddonRef.current) return;

    const fit = () => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        /* noop */
      }
    };

    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, [active]);

  // Focus terminal when active
  useEffect(() => {
    if (active && xtermRef.current) {
      xtermRef.current.focus();
    }
  }, [active]);

  return (
    <div
      className={clsx("absolute inset-0 rounded-b-lg overflow-hidden", active ? "block" : "hidden")}
      ref={termRef}
    />
  );
}

// ---------------------------------------------------------------------------
// Terminal Page – tab management, container quick-connect
// ---------------------------------------------------------------------------

export default function TerminalPage() {
  const { containers, fetchContainers } = useContainerStore();
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [{ id: nextTabId(), label: "HyperBox" }]);
  const [activeTab, setActiveTab] = useState(tabs[0].id);
  const [showContainerPicker, setShowContainerPicker] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);

  const runningContainers = containers.filter((c) => c.status === "running");

  useEffect(() => {
    fetchContainers();
  }, [fetchContainers]);

  const addTab = useCallback((containerId?: string, containerName?: string) => {
    const id = nextTabId();
    const label = containerName
      ? containerName.slice(0, 16)
      : containerId
        ? containerId.slice(0, 12)
        : "HyperBox";
    setTabs((prev) => [...prev, { id, label, containerId, containerName }]);
    setActiveTab(id);
    setShowContainerPicker(false);
  }, []);

  const closeTab = useCallback(
    (tabId: string) => {
      setTabs((prev) => {
        const filtered = prev.filter((t) => t.id !== tabId);
        if (filtered.length === 0) {
          const fresh = { id: nextTabId(), label: "HyperBox" };
          setActiveTab(fresh.id);
          return [fresh];
        }
        if (activeTab === tabId) {
          setActiveTab(filtered[filtered.length - 1].id);
        }
        return filtered;
      });
    },
    [activeTab],
  );

  return (
    <div
      className={clsx(
        "flex flex-col",
        isMaximized ? "fixed inset-0 z-50 bg-gray-900" : "h-[calc(100vh-8rem)]",
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Terminal</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Interactive container management console
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsMaximized(!isMaximized)}
            className="btn btn-secondary p-2"
            title={isMaximized ? "Restore" : "Maximize"}
          >
            {isMaximized ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
          </button>
        </div>
      </div>

      {/* Terminal Chrome */}
      <div className="flex-1 flex flex-col bg-[#0D1117] rounded-lg border border-gray-700 overflow-hidden min-h-0">
        {/* Tab Bar */}
        <div className="flex items-center bg-[#161B22] border-b border-gray-700 px-2">
          <div className="flex items-center gap-1 flex-1 overflow-x-auto scrollbar-none py-1">
            {tabs.map((tab) => (
              <div
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={clsx(
                  "flex items-center gap-2 px-3 py-1.5 rounded-md text-sm cursor-pointer transition-colors whitespace-nowrap group",
                  activeTab === tab.id
                    ? "bg-[#0D1117] text-white"
                    : "text-gray-400 hover:text-gray-200 hover:bg-gray-800",
                )}
              >
                {tab.containerId ? (
                  <Box className="w-3.5 h-3.5 text-cyan-400" />
                ) : (
                  <TerminalIcon className="w-3.5 h-3.5 text-green-400" />
                )}
                <span>{tab.label}</span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(tab.id);
                  }}
                  className="ml-1 opacity-0 group-hover:opacity-100 transition-opacity text-gray-500 hover:text-white"
                >
                  <X className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>

          {/* New tab actions */}
          <div className="flex items-center gap-1 ml-2 border-l border-gray-700 pl-2">
            <button
              onClick={() => addTab()}
              className="p-1.5 text-gray-400 hover:text-white rounded transition-colors"
              title="New terminal"
            >
              <Plus className="w-4 h-4" />
            </button>
            <div className="relative">
              <button
                onClick={() => {
                  fetchContainers();
                  setShowContainerPicker(!showContainerPicker);
                }}
                className="p-1.5 text-gray-400 hover:text-white rounded transition-colors"
                title="Connect to container"
              >
                <Box className="w-4 h-4" />
              </button>

              {showContainerPicker && (
                <div className="absolute right-0 top-full mt-1 w-64 bg-gray-800 border border-gray-700 rounded-lg shadow-xl z-50 overflow-hidden">
                  <div className="px-3 py-2 border-b border-gray-700 text-xs font-medium text-gray-400 uppercase tracking-wider">
                    Running Containers
                  </div>
                  {runningContainers.length === 0 ? (
                    <div className="px-3 py-4 text-sm text-gray-500 text-center">
                      No running containers
                    </div>
                  ) : (
                    runningContainers.map((c) => (
                      <button
                        key={c.id}
                        onClick={() => addTab(c.id, c.name)}
                        className="w-full px-3 py-2 text-left text-sm text-gray-300 hover:bg-gray-700 flex items-center gap-2 transition-colors"
                      >
                        <Box className="w-3.5 h-3.5 text-cyan-400 flex-shrink-0" />
                        <div className="truncate">
                          <p className="font-medium truncate">{c.name || c.id.slice(0, 12)}</p>
                          <p className="text-xs text-gray-500 truncate">{c.image}</p>
                        </div>
                      </button>
                    ))
                  )}
                  <div className="px-3 py-2 border-t border-gray-700">
                    <button
                      onClick={() => fetchContainers()}
                      className="text-xs text-gray-400 hover:text-white flex items-center gap-1 transition-colors"
                    >
                      <RefreshCw className="w-3 h-3" />
                      Refresh
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Terminal Instances */}
        <div className="flex-1 relative min-h-0">
          {tabs.map((tab) => (
            <TerminalInstance key={tab.id} tab={tab} active={activeTab === tab.id} />
          ))}
        </div>
      </div>
    </div>
  );
}
