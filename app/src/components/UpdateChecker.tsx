import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, X, RefreshCw, CheckCircle, AlertCircle } from "lucide-react";
import { clsx } from "clsx";

interface UpdateInfo {
  version: string;
  current_version: string;
  body: string | null;
  date: string | null;
  available: boolean;
}

interface UpdateProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  status: string;
}

type UpdateState = "idle" | "checking" | "available" | "downloading" | "ready" | "error";

export default function UpdateChecker() {
  const [state, setState] = useState<UpdateState>("idle");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  const checkForUpdates = useCallback(async () => {
    setState("checking");
    setError(null);
    
    try {
      const info = await invoke<UpdateInfo>("check_for_updates");
      setUpdateInfo(info);
      
      if (info.available) {
        setState("available");
      } else {
        setState("idle");
      }
    } catch (err) {
      console.error("Failed to check for updates:", err);
      setError(err instanceof Error ? err.message : String(err));
      setState("error");
    }
  }, []);

  const installUpdate = useCallback(async () => {
    setState("downloading");
    setProgress({ downloaded: 0, total: null, percent: 0, status: "Starting download..." });
    
    try {
      // Note: In a real implementation, we'd use Tauri events for progress
      await invoke("install_update");
      setState("ready");
      setProgress({ downloaded: 100, total: 100, percent: 100, status: "Ready to restart" });
    } catch (err) {
      console.error("Failed to install update:", err);
      setError(err instanceof Error ? err.message : String(err));
      setState("error");
    }
  }, []);

  // Check for updates on mount and every 6 hours
  useEffect(() => {
    checkForUpdates();
    
    const interval = setInterval(checkForUpdates, 6 * 60 * 60 * 1000);
    return () => clearInterval(interval);
  }, [checkForUpdates]);

  // Don't render if dismissed or no update available
  if (dismissed || state === "idle") {
    return null;
  }

  const handleDismiss = () => {
    setDismissed(true);
  };

  return (
    <div
      className={clsx(
        "fixed bottom-4 right-4 max-w-sm w-full",
        "bg-white dark:bg-gray-800 rounded-lg shadow-lg",
        "border border-gray-200 dark:border-gray-700",
        "p-4 z-50 animate-slide-up"
      )}
    >
      <div className="flex items-start gap-3">
        {/* Icon */}
        <div
          className={clsx(
            "flex-shrink-0 w-10 h-10 rounded-full flex items-center justify-center",
            state === "checking" && "bg-blue-100 dark:bg-blue-900/30",
            state === "available" && "bg-primary-100 dark:bg-primary-900/30",
            state === "downloading" && "bg-amber-100 dark:bg-amber-900/30",
            state === "ready" && "bg-success-100 dark:bg-success-900/30",
            state === "error" && "bg-error-100 dark:bg-error-900/30"
          )}
        >
          {state === "checking" && (
            <RefreshCw className="w-5 h-5 text-blue-600 dark:text-blue-400 animate-spin" />
          )}
          {state === "available" && (
            <Download className="w-5 h-5 text-primary-600 dark:text-primary-400" />
          )}
          {state === "downloading" && (
            <RefreshCw className="w-5 h-5 text-amber-600 dark:text-amber-400 animate-spin" />
          )}
          {state === "ready" && (
            <CheckCircle className="w-5 h-5 text-success-600 dark:text-success-400" />
          )}
          {state === "error" && (
            <AlertCircle className="w-5 h-5 text-error-600 dark:text-error-400" />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
            {state === "checking" && "Checking for updates..."}
            {state === "available" && "Update Available"}
            {state === "downloading" && "Downloading Update..."}
            {state === "ready" && "Update Ready"}
            {state === "error" && "Update Error"}
          </h3>
          
          <p className="mt-1 text-xs text-gray-600 dark:text-gray-400">
            {state === "checking" && "Please wait while we check for new versions."}
            {state === "available" && updateInfo && (
              <>
                Version <span className="font-mono">{updateInfo.version}</span> is available.
                {updateInfo.body && (
                  <span className="block mt-1 text-gray-500">
                    {updateInfo.body.slice(0, 100)}
                    {updateInfo.body.length > 100 && "..."}
                  </span>
                )}
              </>
            )}
            {state === "downloading" && progress && (
              <>
                {progress.status}
                {progress.total && (
                  <span className="block mt-1">
                    {Math.round(progress.downloaded / 1024)} KB / {Math.round(progress.total / 1024)} KB
                  </span>
                )}
              </>
            )}
            {state === "ready" && "Restart the app to apply the update."}
            {state === "error" && error}
          </p>

          {/* Progress bar */}
          {state === "downloading" && progress && (
            <div className="mt-2 w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
              <div
                className="bg-primary-500 h-1.5 rounded-full transition-all duration-300"
                style={{ width: `${progress.percent}%` }}
              />
            </div>
          )}

          {/* Actions */}
          <div className="mt-3 flex gap-2">
            {state === "available" && (
              <button
                onClick={installUpdate}
                className="btn btn-sm btn-primary"
              >
                <Download className="w-4 h-4 mr-1" />
                Install Update
              </button>
            )}
            {state === "ready" && (
              <button
                onClick={() => window.location.reload()}
                className="btn btn-sm btn-success"
              >
                <RefreshCw className="w-4 h-4 mr-1" />
                Restart Now
              </button>
            )}
            {state === "error" && (
              <button
                onClick={checkForUpdates}
                className="btn btn-sm btn-outline"
              >
                <RefreshCw className="w-4 h-4 mr-1" />
                Retry
              </button>
            )}
          </div>
        </div>

        {/* Dismiss button */}
        {(state === "available" || state === "error") && (
          <button
            onClick={handleDismiss}
            className="flex-shrink-0 p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
          >
            <X className="w-4 h-4 text-gray-400" />
          </button>
        )}
      </div>
    </div>
  );
}

// Settings page section for manual update checks
export function UpdateSettings() {
  const [state, setState] = useState<"idle" | "checking" | "available" | "upToDate">("idle");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Get current version on mount
    invoke<{ version: string }>("get_app_version")
      .then((info) => setCurrentVersion(info.version))
      .catch(console.error);
  }, []);

  const checkForUpdates = async () => {
    setState("checking");
    setError(null);
    
    try {
      const info = await invoke<UpdateInfo>("check_for_updates");
      setUpdateInfo(info);
      setState(info.available ? "available" : "upToDate");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setState("idle");
    }
  };

  return (
    <div className="card p-6">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Software Updates
      </h3>
      
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            Current Version: <span className="font-mono">{currentVersion || "Loading..."}</span>
          </p>
          {state === "available" && updateInfo && (
            <p className="text-sm text-primary-600 dark:text-primary-400 mt-1">
              New version available: <span className="font-mono">{updateInfo.version}</span>
            </p>
          )}
          {state === "upToDate" && (
            <p className="text-sm text-success-600 dark:text-success-400 mt-1">
              You're up to date!
            </p>
          )}
          {error && (
            <p className="text-sm text-error-600 dark:text-error-400 mt-1">
              {error}
            </p>
          )}
        </div>
        
        <button
          onClick={checkForUpdates}
          disabled={state === "checking"}
          className="btn btn-outline"
        >
          {state === "checking" ? (
            <>
              <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
              Checking...
            </>
          ) : (
            <>
              <RefreshCw className="w-4 h-4 mr-2" />
              Check for Updates
            </>
          )}
        </button>
      </div>
    </div>
  );
}

#[cfg(test)]
mod tests {
    // Note: React component tests would go here using @testing-library/react
    // For now, we rely on manual testing and TypeScript type checking
}
