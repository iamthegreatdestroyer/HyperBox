import { NavLink, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  FolderKanban,
  Box,
  Layers,
  Gauge,
  Settings,
  Zap,
  Circle,
} from "lucide-react";
import { useSystemStore } from "../stores/system";
import { clsx } from "clsx";

interface LayoutProps {
  children: React.ReactNode;
}

const navigation = [
  { name: "Dashboard", href: "/", icon: LayoutDashboard },
  { name: "Projects", href: "/projects", icon: FolderKanban },
  { name: "Containers", href: "/containers", icon: Box },
  { name: "Images", href: "/images", icon: Layers },
  { name: "Performance", href: "/performance", icon: Gauge },
  { name: "Settings", href: "/settings", icon: Settings },
];

export default function Layout({ children }: LayoutProps) {
  const location = useLocation();
  const { daemonConnected, systemInfo } = useSystemStore();

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      {/* Sidebar */}
      <aside className="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
        {/* Logo */}
        <div className="h-16 flex items-center gap-3 px-6 border-b border-gray-200 dark:border-gray-700">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
            <Zap className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-bold text-gray-900 dark:text-white">HyperBox</h1>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              v{systemInfo?.version || "0.1.0"}
            </p>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-4 py-4 space-y-1">
          {navigation.map((item) => {
            const isActive = location.pathname === item.href;
            return (
              <NavLink
                key={item.name}
                to={item.href}
                className={clsx("sidebar-item", isActive && "active")}
              >
                <item.icon className="w-5 h-5" />
                {item.name}
              </NavLink>
            );
          })}
        </nav>

        {/* Daemon Status */}
        <div className="p-4 border-t border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <Circle
              className={clsx(
                "w-3 h-3",
                daemonConnected
                  ? "fill-success-500 text-success-500"
                  : "fill-error-500 text-error-500",
              )}
            />
            <span className="text-sm text-gray-600 dark:text-gray-400">
              {daemonConnected ? "Daemon Connected" : "Daemon Offline"}
            </span>
          </div>
          {systemInfo && (
            <div className="mt-2 text-xs text-gray-500 dark:text-gray-500">
              <div>{systemInfo.containersRunning} containers running</div>
              <div>{systemInfo.images} images</div>
            </div>
          )}
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-auto">
        <div className="p-8">{children}</div>
      </main>
    </div>
  );
}
