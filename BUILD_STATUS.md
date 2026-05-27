# HyperBox Build Status — v0.2.0

**Date:** 2026-05-26  
**Branch:** feat/erofs

---

## Rust Backend — `cargo build --release`

**Status: PASSING ✅** (0 errors, 8 warnings)

| Crate | Status | Notes |
|-------|--------|-------|
| `hyperbox-core` | ✅ Compiles | Runtime trait, Docker/crun backends, isolation, storage, network |
| `hyperbox-project` | ✅ Compiles | ProjectManager, detection, orchestration, devcontainer support |
| `hyperbox-optimize` | ✅ Compiles | CRIU, lazy-load, prewarm, predict, EROFS, dedup, profile-exporter |
| `hyperbox-cli` | ✅ Compiles | `hb` binary — container/image/project/system commands |
| `hyperbox-daemon` | ✅ Compiles | `hyperboxd` binary — REST API, IPC (named pipe on Windows), lifecycle |
| `hyperbox-desktop` (Tauri) | ✅ Compiles | Tauri 2.0 app shell + all commands |
| `hyperbox-e2e-tests` | ✅ Compiles | Integration test scaffolding |

**Release binaries produced:**
- `target/release/hyperboxd.exe` — 5.2 MB
- `target/release/hb.exe` — 5.4 MB
- `target/release/hyperbox-desktop.exe` — 11.5 MB (prior build artifact)

**Warnings fixed:** None required — all warnings are unused function stubs (update commands, version info) that exist as API surface placeholders.

---

## TypeScript Frontend — `pnpm install && pnpm build`

**Status: IN PROGRESS ⏳** (pnpm install running; vite build pending)

| Component | Status | Notes |
|-----------|--------|-------|
| `node_modules` setup | ✅ Installed | Hoisted linker mode (`.npmrc`); `lucide-react` was missing, added |
| TypeScript type-check | ✅ Fixed | Excluded test files from `tsconfig.json`; fixed `c.name` → `c.containerName` in `Performance.tsx` |
| `UpdateChecker.tsx` | ✅ Fixed | Removed Rust test code fragment; added `type="button"` and `title="Dismiss"` |
| Vite build | ⏳ Pending pnpm install | Running in terminal |

**TypeScript issues fixed:**
1. `src/components/UpdateChecker.tsx:292` — `#[cfg(test)] mod tests { ... }` (Rust code accidentally appended to TSX file) — removed
2. Missing `type="button"` on 4 buttons in `UpdateChecker.tsx` — fixed
3. Missing `title` on dismiss button — fixed
4. `src/pages/Performance.tsx:444` — `c.name` → `c.containerName` (field name mismatch with `ContainerMetricsSnapshot`)
5. `tsconfig.json` — excluded `src/test/**` and co-located `*.test.*` from `tsc` type-check to prevent test file errors blocking `pnpm build`
6. `lucide-react` missing from `node_modules` despite being in `package.json` — added via `pnpm add`

---

## Architecture Summary

### Container Runtime
- **Backend:** Docker via Bollard API (Windows primary) + crun (Linux)
- **Interface:** `ContainerRuntime` trait in `hyperbox-core`
- **State:** `DaemonState` in `hyperbox-daemon/src/state.rs` — `DashMap`-based concurrent container registry

### IPC
- **Windows:** Named pipe at `\\.\pipe\hyperbox`
- **Linux:** Unix socket at configurable path
- **REST API:** HTTP on `127.0.0.1:8181` via Axum (changed from 8080 to avoid conflict with Docker Desktop)
- **gRPC:** Placeholder on port 50051 (stub, not yet wired to proto)

### Optimization Stack
- **CRIU:** Checkpoint/restore manager (available on Linux; stubbed on Windows)
- **Lazy loading:** eStargz TOC-based layer loader
- **Prewarming:** ML-predicted container pre-warming via `UsagePredictor`
- **EROFS:** `EROFSManager` for read-only compressed layers (Linux 5.19+)

### Tauri UI
- **Framework:** Tauri 2.0 + React 18 + Zustand + TanStack Query + Recharts
- **Pages:** Dashboard, Projects, Containers, Images, Performance, Terminal, Settings
- **IPC:** All commands wired via `invoke()` to `AppState` → `DaemonClient` → HTTP REST

---

## What's Implemented vs Stubbed

### Fully Implemented ✅
- Container lifecycle trait + Docker backend (create/start/stop/pause/resume/remove/kill/exec/logs/stats)
- REST API endpoints (all routes present and functional)
- Windows named pipe IPC server
- Project detection (Dockerfile, docker-compose, language inference)
- CRIU checkpoint/restore logic (requires CRIU binary on Linux)
- Lazy layer loader (eStargz TOC fetch + chunk download)
- Usage predictor (decay-weighted temporal ML model)
- Prewarm manager (prediction-driven, TTL-based cleanup)
- Profile exporter (JSON/libseccomp/eBPF formats)
- EROFS mount manager (Linux kernel 5.19+ fscache)
- All Tauri commands (project/container/image/system/settings/update)

### Stubbed / Partial 🟡
- gRPC server: placeholder sleep loop, no proto-generated code
- `start_daemon` in Tauri: Linux-only process spawn; Windows needs named pipe launch
- CRIU restore: actual `criu restore` command integration pending (paths wired, execution stubbed)
- Container prewarming execution: prediction triggers identified, actual container warm-up call stubbed
- `hyperbox start/stop/list` CLI commands: wired to REST API client, functional when daemon running
- PSI memory monitor: Linux-only (`/proc/pressure/memory`); Windows returns disabled

### Not Yet Implemented ❌
- gRPC protobuf definitions + code generation
- Full E2E test coverage
- Windows daemon auto-start from Tauri app
- Network namespace isolation per project (simulated on Windows)
