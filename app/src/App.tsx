import { Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Projects from "./pages/Projects";
import Containers from "./pages/Containers";
import Images from "./pages/Images";
import Performance from "./pages/Performance";
import Terminal from "./pages/Terminal";
import Settings from "./pages/Settings";
import { useSystemStore } from "./stores/system";
import { useEffect } from "react";

function App() {
  const { checkDaemonStatus, theme } = useSystemStore();

  useEffect(() => {
    // Check daemon status on load
    checkDaemonStatus();

    // Apply theme
    if (
      theme === "dark" ||
      (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches)
    ) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [checkDaemonStatus, theme]);

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/projects" element={<Projects />} />
        <Route path="/containers" element={<Containers />} />
        <Route path="/images" element={<Images />} />
        <Route path="/performance" element={<Performance />} />
        <Route path="/terminal" element={<Terminal />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Layout>
  );
}

export default App;
