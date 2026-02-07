# 🧬 HYPERBOX EVOLUTION: MASTER CLASS AUTONOMOUS BUILD DIRECTIVE v2.0

**Classification:** Maximum Autonomy Enhancement Protocol  
**Project:** HyperBox - 20× Docker Desktop Replacement  
**Repository:** `S:\HyperBox` → `https://github.com/iamthegreatdestroyer/HyperBox`  
**Execution Mode:** Elite Agent Collective — Additive Enhancement Framework  
**Supersedes:** `HyperBox-MasterClass-Prompt.md` (v1.0 — Genesis/Scaffolding Phase)  
**Effective Date:** 2026-02-06  

---

## ⚠️ PRIME DIRECTIVE: INTEGRITY-FIRST EVOLUTION

**You are now operating as @OMNISCIENT Master Coordinator for HyperBox Phase 2: Evolution.**

The project has completed Phase 1 (Genesis/Scaffolding). A comprehensive 6-crate Rust workspace, Tauri desktop app, CLI, daemon, and CI/CD pipeline are **already in place and compiling**. Your mandate is NOT to rebuild — it is to **evolve, enhance, and complete** the existing architecture with cutting-edge innovations discovered through extensive research.

### 🔴 THE GOLDEN RULE: DO NO HARM

```
┌──────────────────────────────────────────────────────────────────────────┐
│  INTEGRITY PRESERVATION PROTOCOL                                         │
│                                                                          │
│  1. NEVER delete, overwrite, or restructure existing files blindly       │
│  2. ALWAYS read a file's current contents BEFORE modifying it            │
│  3. ALWAYS preserve existing trait signatures, public APIs, and types    │
│  4. ADD new modules alongside existing ones — do not replace             │
│  5. NEW dependencies go in ADDITION to existing Cargo.toml entries       │
│  6. RUN `cargo check --workspace` after EVERY significant change         │
│  7. If `cargo check` fails, FIX the error before proceeding             │
│  8. COMMIT working states frequently with descriptive messages           │
│  9. Create feature branches for major additions: `feat/<feature-name>`   │
│  10. When in doubt, ADD a new file rather than modify an existing one    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Pre-Modification Protocol (Execute EVERY Time)

Before touching ANY existing file, execute this sequence:

```
STEP 1: Read the file completely
STEP 2: Identify all public exports, trait implementations, and type definitions
STEP 3: Plan your changes as ADDITIVE modifications
STEP 4: Make changes
STEP 5: Run `cargo check -p <affected-crate>`
STEP 6: If check passes, continue. If not, revert and fix.
```

---

## 📊 CURRENT PROJECT STATE (As of v2.0 Directive)

### What Exists (DO NOT REBUILD)

```
S:\HyperBox\
├── Cargo.toml                          ✅ Workspace with 6+1 member crates
├── crates/
│   ├── hyperbox-core/src/
│   │   ├── lib.rs                      ✅ Core library exports
│   │   ├── types.rs                    ✅ ContainerId, ContainerSpec, etc.
│   │   ├── error.rs                    ✅ CoreError enum
│   │   ├── runtime/
│   │   │   ├── mod.rs                  ✅ Runtime module exports
│   │   │   ├── traits.rs              ✅ ContainerRuntime trait (comprehensive)
│   │   │   ├── crun.rs                ✅ CrunRuntime impl (scaffolded)
│   │   │   ├── docker.rs             ✅ DockerRuntime via bollard
│   │   │   └── registry.rs           ✅ Image registry client
│   │   ├── isolation/
│   │   │   ├── mod.rs                 ✅ Isolation module exports
│   │   │   ├── cgroups.rs            ✅ CgroupManager scaffolded
│   │   │   ├── namespaces.rs         ✅ Namespace management
│   │   │   └── seccomp.rs            ✅ Seccomp profiles
│   │   ├── storage/
│   │   │   ├── mod.rs                 ✅ Storage module exports
│   │   │   ├── composefs.rs          ✅ Composefs scaffolded
│   │   │   ├── layers.rs             ✅ Layer management
│   │   │   └── registry.rs           ✅ Registry storage
│   │   └── network/
│   │       ├── mod.rs                 ✅ Network module exports
│   │       ├── bridge.rs             ✅ Bridge networking
│   │       ├── cni.rs                ✅ CNI integration
│   │       └── ports.rs              ✅ Port management
│   ├── hyperbox-project/src/
│   │   ├── lib.rs                     ✅ Project library
│   │   ├── manager.rs                ✅ ProjectManager
│   │   ├── config.rs                 ✅ ProjectConfig
│   │   ├── detection.rs              ✅ Project detection
│   │   ├── ports.rs                  ✅ PortAllocator
│   │   ├── resources.rs              ✅ Resource management
│   │   ├── orchestration.rs          ✅ Container orchestration
│   │   ├── watcher.rs                ✅ File watcher (notify-based)
│   │   └── error.rs                  ✅ Error types
│   ├── hyperbox-optimize/src/
│   │   ├── lib.rs                     ✅ Optimize library
│   │   ├── criu.rs                   ✅ CriuManager scaffolded
│   │   ├── lazy_load.rs              ✅ LazyImageLoader scaffolded
│   │   ├── predict.rs                ✅ UsagePredictor scaffolded
│   │   ├── prewarm.rs                ✅ PrewarmManager scaffolded
│   │   └── error.rs                  ✅ Error types
│   ├── hyperbox-cli/src/
│   │   ├── main.rs                    ✅ CLI entrypoint
│   │   ├── client.rs                 ✅ IPC client
│   │   └── commands/                 ✅ Command modules
│   ├── hyperbox-daemon/src/
│   │   ├── main.rs                    ✅ Daemon entrypoint
│   │   ├── api.rs                    ✅ API routes
│   │   ├── ipc.rs                    ✅ IPC server
│   │   ├── grpc.rs                   ✅ gRPC service
│   │   ├── state.rs                  ✅ Application state
│   │   ├── config.rs                 ✅ Configuration
│   │   ├── health.rs                 ✅ Health checks
│   │   ├── lifecycle.rs              ✅ Lifecycle management
│   │   └── error.rs                  ✅ Error types
├── app/                               ✅ Tauri 2.0 + React frontend
│   ├── src-tauri/src/
│   │   ├── main.rs                   ✅ Tauri entrypoint
│   │   ├── lib.rs                    ✅ Library exports
│   │   ├── commands.rs               ✅ Tauri commands
│   │   ├── state.rs                  ✅ App state
│   │   └── daemon.rs                ✅ Daemon bridge
│   └── src/
│       ├── App.tsx                   ✅ React root
│       ├── pages/                    ✅ Dashboard, Containers, Projects, etc.
│       ├── components/               ✅ Layout, CreateContainerModal, etc.
│       └── stores/                   ✅ State management
├── .github/
│   ├── workflows/                    ✅ CI/CD pipelines
│   ├── agents/                       ✅ Agent configurations
│   └── copilot-instructions.md       ✅ Elite Agent Collective v3.0
└── tests/                            ✅ Test scaffolding
```

### What Needs Enhancement (YOUR MISSION)

The existing code is approximately:
- **~65% scaffolded** (types, traits, module structure, error handling)
- **~25% functionally implemented** (partial runtime, basic IPC, UI skeleton)
- **~10% production-ready** (build system, CI/CD, configuration)

Your job is to take the scaffolded implementations and fill them with **real, working, production-grade code** incorporating the innovations below.

---

## 🚀 EVOLUTION TECHNOLOGY STACK

### Research-Validated Innovations to Integrate

The following technologies have been extensively researched and validated. Each entry includes the target file, rationale, and implementation approach.

---

### EVOLUTION TRACK 1: RUNTIME LAYER [REF:EVO-RT-100]

#### 1A. youki Integration (Rust-Native OCI Runtime) [REF:EVO-RT-101]

**Target:** `crates/hyperbox-core/src/runtime/youki.rs` ← NEW FILE  
**Why:** youki is a Rust-native OCI runtime (CNCF Sandbox, ~6.8k stars). Unlike crun (C binary invoked via subprocess), youki's `libcontainer` crate can be linked directly as a Rust library, eliminating subprocess spawn overhead entirely.

**Implementation:**

```rust
// NEW FILE: crates/hyperbox-core/src/runtime/youki.rs

// 1. Add to workspace Cargo.toml under [workspace.dependencies]:
//    libcontainer = "0.5"  (or latest youki-libcontainer version)

// 2. Add to hyperbox-core/Cargo.toml:
//    libcontainer = { workspace = true, optional = true }
//    [features]
//    youki = ["libcontainer"]

// 3. Implement YoukiRuntime that implements the existing ContainerRuntime trait
//    from crates/hyperbox-core/src/runtime/traits.rs
//    
//    Key advantage: Direct library calls vs Command::new("crun")
//    Expected: Eliminate ~15-20ms subprocess overhead per operation

// 4. Register in crates/hyperbox-core/src/runtime/mod.rs:
//    #[cfg(feature = "youki")]
//    pub mod youki;
```

**Agent Assignment:** @APEX primary, @CORE support  
**Priority:** P0 — Highest impact runtime improvement  
**Validation:** `cargo test -p hyperbox-core --features youki test_youki_lifecycle`

#### 1B. WASM Runtime Support (Market Opportunity) [REF:EVO-RT-102]

**Target:** `crates/hyperbox-core/src/runtime/wasm.rs` ← NEW FILE  
**Why:** Docker has **deprecated and is removing** WASM support from Desktop. This is a direct market opening. containerd runwasi v1.0 GA, WASI 0.2 Component Model stable. Fermyon Spin achieves 0.52ms cold starts vs Docker's 2.8-3.5 seconds — a 1,000× improvement.

**Implementation:**

```rust
// NEW FILE: crates/hyperbox-core/src/runtime/wasm.rs

// 1. Add dependencies:
//    wasmtime = "latest"
//    wasmtime-wasi = "latest"

// 2. Implement WasmRuntime that implements ContainerRuntime trait
//    - Detect .wasm artifacts vs OCI images  
//    - Use Wasmtime with AOT compilation for <1ms startup
//    - Support WASI 0.2 Component Model
//    - Implement resource limits via Wasmtime's fuel metering

// 3. Key methods:
//    - create(): Pre-compile WASM module, cache compiled artifact
//    - start(): Instantiate from cached compilation (<1ms)
//    - exec(): Run exported WASM functions
//    - stats(): Report fuel consumption as CPU proxy

// 4. Register behind feature flag:
//    #[cfg(feature = "wasm")]
//    pub mod wasm;
```

**Agent Assignment:** @APEX primary, @NEXUS support  
**Priority:** P1 — Strategic competitive advantage  
**Validation:** `cargo test -p hyperbox-core --features wasm test_wasm_hello_world`

#### 1C. Hybrid Runtime Selector [REF:EVO-RT-103]

**Target:** `crates/hyperbox-core/src/runtime/selector.rs` ← NEW FILE  
**Why:** Automatically route workloads to optimal runtime (youki for OCI, Wasmtime for WASM, crun as fallback).

```rust
// NEW FILE: crates/hyperbox-core/src/runtime/selector.rs

// RuntimeSelector inspects the ContainerSpec and routes to the best runtime:
// - spec.image ends with .wasm → WasmRuntime
// - spec.runtime_hint == "youki" → YoukiRuntime  
// - spec.runtime_hint == "crun" → CrunRuntime
// - Default: YoukiRuntime (Rust-native, no subprocess overhead)
// Implements ContainerRuntime trait itself as a delegating proxy.
```

**Agent Assignment:** @ARCHITECT  
**Priority:** P1  

---

### EVOLUTION TRACK 2: INSTANT RESTORE [REF:EVO-RS-200]

#### 2A. Demand-Paged CRIU Restore [REF:EVO-RS-201]

**Target:** ENHANCE `crates/hyperbox-optimize/src/criu.rs` (existing file)  
**Why:** Standard CRIU restore loads ALL memory pages upfront (hundreds of ms). Demand-paged restore uses `mmap(MAP_PRIVATE)` on snapshot files — pages load on-demand via page faults. Firecracker achieves ~4ms bare restore, ~10ms full Linux system with this technique. This is the single highest-impact optimization for warm starts.

**Implementation Strategy (ADDITIVE to existing CriuManager):**

```rust
// ENHANCE: crates/hyperbox-optimize/src/criu.rs
// ADD these methods to the existing CriuManager impl block:

// 1. Add rust-criu dependency:
//    rust-criu = "0.4"  (wraps CRIU RPC protobuf bindings)

// 2. New method: demand_paged_restore()
//    - mmap checkpoint pages file with MAP_PRIVATE (copy-on-write)
//    - Pages load on-demand via page faults
//    - Pre-fault "hot pages" (entrypoint, shared libs) in background
//    - Target: <10ms restore time

// 3. New method: checkpoint_with_cooperation()
//    - Signal application to flush state before checkpoint
//    - Minimize dirty pages for smaller, faster snapshots

// 4. New method: incremental_checkpoint()
//    - Track dirty pages since last checkpoint
//    - Only store changed pages (90% storage reduction per SnapStart research)

// 5. New struct: HotPageTracker
//    - Record page fault patterns during first restore
//    - Pre-fault those pages on subsequent restores (Snapipeline technique: 53% reduction)

// IMPORTANT: Preserve existing Checkpoint struct, CriuOptions, and CriuManager methods.
// ADD new methods, do not modify existing signatures.
```

**Agent Assignment:** @VELOCITY primary, @CORE support  
**Priority:** P0 — Defines the "instant start" experience  
**Validation:** Benchmark restore time with `criterion`: target <10ms for alpine container

#### 2B. Cooperative Checkpointing (No Root Required) [REF:EVO-RS-202]

**Target:** `crates/hyperbox-optimize/src/cooperative_checkpoint.rs` ← NEW FILE  
**Why:** CRaC (Coordinating Resources at Checkpoint) achieves 4s → 40ms restore WITHOUT root privileges. Pattern is generalizable beyond JVM. Critical for desktop UX where root prompts destroy user experience.

```rust
// NEW FILE: crates/hyperbox-optimize/src/cooperative_checkpoint.rs

// 1. Application cooperation protocol:
//    - SIGUSR2 → application flushes state to known location
//    - Application closes network sockets, file handles
//    - Checkpoint captures minimal state (clean snapshot)
//    - Restore reopens resources from saved state

// 2. No root privileges required (unlike raw CRIU)
// 3. Implement CooperativeCheckpoint trait that containers can opt into
```

**Agent Assignment:** @VELOCITY  
**Priority:** P2  

---

### EVOLUTION TRACK 3: IMAGE FORMAT INNOVATIONS [REF:EVO-IF-300]

#### 3A. Nydus Integration (Rust-Native Lazy Loading) [REF:EVO-IF-301]

**Target:** ENHANCE `crates/hyperbox-optimize/src/lazy_load.rs` (existing file)  
**Why:** Nydus (written in Rust, CNCF) uses RAFS v6 with in-kernel EROFS integration (Linux 5.19+). Eliminates FUSE overhead entirely. Ant Group: near-zero image pull across 10,000 Kubernetes nodes. Datadog: node startup 5 minutes → seconds. **This replaces our eStargz strategy** — Grab measured eStargz causes 5× slower app startup due to runtime decompression overhead.

**Implementation Strategy (ENHANCE existing LazyImageLoader):**

```rust
// ENHANCE: crates/hyperbox-optimize/src/lazy_load.rs

// 1. Add dependency: nydus-rs (or nydus-api, nydus-storage)
//    Check crates.io for latest nydus Rust crates

// 2. Add NydusLoader struct alongside existing LazyImageLoader:
//    - Convert OCI images to Nydus RAFS format on first pull
//    - Cache converted images locally  
//    - Mount via EROFS (kernel-native, zero FUSE overhead)
//    - Chunk-level deduplication across images

// 3. Update LazyImageLoader to prefer Nydus when kernel supports EROFS:
//    - Check kernel version >= 5.19
//    - Fall back to eStargz if EROFS unavailable

// 4. CRITICAL: Keep existing eStargz code as fallback
//    Add NydusLoader as preferred path, not replacement

// PRESERVE: All existing LazyImageLoader methods and types
```

**Agent Assignment:** @VELOCITY primary, @FORGE support  
**Priority:** P0 — Replaces eStargz as primary lazy loading strategy  

#### 3B. composefs Storage Deduplication [REF:EVO-IF-302]

**Target:** ENHANCE `crates/hyperbox-core/src/storage/composefs.rs` (existing file)  
**Why:** composefs (v1.0, kernel 6.5+) provides content-addressed backing store. Multiple images sharing files get the SAME page cache entries. FOSDEM 2024 measurement: 20 copies of 181MB layer = 181MB base + ~2.4MB per copy (vs 3.6GB with overlayfs). 97% of files across Docker Hub images are redundant — composefs eliminates this.

**Implementation Strategy:**

```rust
// ENHANCE: crates/hyperbox-core/src/storage/composefs.rs

// 1. Read existing file first — preserve all current types and methods
// 2. Add composefs mount/unmount operations
// 3. Content-addressed object store with sha256 digest keys
// 4. Manifest generation linking digests to mount paths
// 5. Cross-image deduplication at file level
// 6. Integration with Nydus for lazy-loaded content-addressed chunks
```

**Agent Assignment:** @VELOCITY, @FORGE  
**Priority:** P1  

#### 3C. FastCDC Content-Defined Chunking [REF:EVO-IF-303]

**Target:** `crates/hyperbox-optimize/src/dedup.rs` ← NEW FILE  
**Why:** Sub-linear deduplication using FastCDC (content-defined chunking) achieves >1 GiB/s throughput via fastcdc-rs crate. Combined with bloom filter index: 1.2MB RAM for 1M chunks at 1% FPR = O(1) dedup checks at wire speed.

```rust
// NEW FILE: crates/hyperbox-optimize/src/dedup.rs

// 1. Add dependencies:
//    fastcdc = "3"     # Content-defined chunking at >1 GiB/s
//    probabilistic-collections = "0.7"  # Bloom filters

// 2. Implement ChunkDeduplicator:
//    - FastCDC splits image layers into variable-size chunks
//    - Bloom filter provides O(1) "definitely not seen" / "probably seen" checks
//    - Only store unique chunks, reference shared ones
//    - Wire-speed deduplication: chunk + bloom check = O(1) per chunk

// 3. Implement ContentMerkleTree:
//    - Content-Defined Merkle Trees (CDMT) for logarithmic image diff
//    - Determine changes between image versions in O(log n) time
//    - 10-15% better deduplication than standard Merkle trees
```

**Agent Assignment:** @VELOCITY primary, @AXIOM support  
**Priority:** P1  

---

### EVOLUTION TRACK 4: KERNEL ACCELERATION [REF:EVO-KA-400]

#### 4A. netkit Container Networking [REF:EVO-KA-401]

**Target:** `crates/hyperbox-core/src/network/netkit.rs` ← NEW FILE  
**Why:** netkit (kernel 6.8+) is purpose-built for container networking, replacing veth pairs. Reduces container networking overhead to ZERO (previously ~35% performance drop vs host). ByteDance deployed across 1 million servers.

```rust
// NEW FILE: crates/hyperbox-core/src/network/netkit.rs

// 1. Detect kernel support (>= 6.8)
// 2. Create netkit devices instead of veth pairs when available
// 3. Attach eBPF programs for packet filtering
// 4. Fall back to bridge.rs/cni.rs when kernel too old
// 5. Implement NetworkBackend trait and register alongside existing implementations
```

**Agent Assignment:** @CORE, @CIPHER  
**Priority:** P2  

#### 4B. io_uring for Internal I/O [REF:EVO-KA-402]

**Target:** `crates/hyperbox-optimize/src/io_accel.rs` ← NEW FILE  
**Why:** io_uring provides ~60% I/O improvement over epoll. Use INTERNALLY for image extraction, tar operations, layer copying. CAUTION: Do NOT expose to containers by default (Google: 60% of 2022 kernel exploits targeted io_uring).

```rust
// NEW FILE: crates/hyperbox-optimize/src/io_accel.rs

// 1. Add dependency: tokio-uring = "0.5" (or io-uring crate)
// 2. Accelerate internal operations only:
//    - Image layer extraction (tar decompression)
//    - Checkpoint file I/O  
//    - Layer copying during composefs operations
// 3. NEVER enable io_uring inside containers (blocked by default seccomp)
// 4. Feature-gated: #[cfg(feature = "io-uring")]
```

**Agent Assignment:** @VELOCITY, @CORE  
**Priority:** P2 — Performance boost for internal operations  

---

### EVOLUTION TRACK 5: MEMORY & RESOURCE OPTIMIZATION [REF:EVO-MR-500]

#### 5A. Dynamic VM Memory Management [REF:EVO-MR-501]

**Target:** `crates/hyperbox-optimize/src/memory.rs` ← NEW FILE  
**Why:** OrbStack pattern (Aug 2024): Track which VM RAM pages are actually in use, release unused portions back to host OS. Solves Docker Desktop's chronic VM memory bloat. Transformative for dev workloads with spike/idle patterns.

```rust
// NEW FILE: crates/hyperbox-optimize/src/memory.rs

// 1. DynamicMemoryManager:
//    - Monitor RSS vs actual usage per container via cgroup memory.current
//    - Implement virtio-balloon protocol for VM memory reclaim
//    - Free page reporting: kernel reports unused pages back
//    - Aggressive reclaim during idle periods
//    - Gradual expansion during load spikes

// 2. Per-Process KSM (kernel 6.4+):
//    - Enable MMF_VM_MERGE_ANY on container processes
//    - 10-50% memory savings for similar containers
//    - Desktop use: Spectre/Meltdown side-channel concerns minimal
//    - Feature-gated: #[cfg(feature = "ksm")]
```

**Agent Assignment:** @VELOCITY, @CORE  
**Priority:** P1 — Directly addresses Docker Desktop's worst UX issue  

#### 5B. Persistent HAMT State Management [REF:EVO-MR-502]

**Target:** `crates/hyperbox-core/src/storage/hamt.rs` ← NEW FILE  
**Why:** Hash Array Mapped Tries provide O(log₃₂ n) ≈ O(1) amortized operations with structural sharing. Each mutation creates new root while sharing ~97% of structure. Zero-cost snapshots, lock-free reads, instant undo/rollback. IPFS uses HAMTs for directory indexing. hamt-rs benchmarks comparable to std::HashMap.

```rust
// NEW FILE: crates/hyperbox-core/src/storage/hamt.rs

// 1. Add dependency: hamt-rs (or im = "15" for persistent collections)
// 2. Use for:
//    - Container state registry (instant snapshots of all container states)
//    - Image layer tree (structural sharing between similar images)  
//    - Configuration versioning (zero-cost config history)
//    - Lock-free concurrent reads from UI/CLI/daemon simultaneously
// 3. Replace DashMap where snapshot capability is valuable
```

**Agent Assignment:** @VELOCITY, @AXIOM  
**Priority:** P2  

---

### EVOLUTION TRACK 6: SECURITY (ROOTLESS-FIRST) [REF:EVO-SC-600]

#### 6A. Landlock Sandboxing [REF:EVO-SC-601]

**Target:** `crates/hyperbox-core/src/isolation/landlock.rs` ← NEW FILE  
**Why:** Landlock LSM (ABI v6, kernel 6.12) provides filesystem + network + IPC + signal sandboxing callable from UNPRIVILEGED processes. No administrator setup, no profile compilation. Near-zero overhead. The landlock Rust crate provides direct integration. This is the cornerstone of the rootless security model.

```rust
// NEW FILE: crates/hyperbox-core/src/isolation/landlock.rs

// 1. Add dependency: landlock = "0.4" (or latest)
// 2. Implement LandlockSandbox:
//    - Filesystem access restrictions per container
//    - Network TCP bind/connect restrictions  
//    - No root privileges required (KEY DIFFERENTIATOR)
//    - Composable with existing seccomp.rs and namespaces.rs
// 3. Auto-generate restrictive rulesets based on container image analysis
// 4. Register in isolation/mod.rs alongside existing seccomp/namespaces
```

**Agent Assignment:** @CIPHER primary, @FORTRESS support  
**Priority:** P0 — Foundational security for rootless operation  

#### 6B. Composable Security Stack [REF:EVO-SC-602]

**Target:** `crates/hyperbox-core/src/isolation/security_stack.rs` ← NEW FILE  
**Why:** Layer all security mechanisms composably, ALL without root:

```
Layer 1: User namespaces + idmap mounts (kernel 5.19+)     ← namespaces.rs (exists)
Layer 2: Landlock LSM (kernel 6.12)                         ← landlock.rs (NEW)
Layer 3: seccomp-bpf + seccomp notify (kernel 5.0+)        ← seccomp.rs (exists)  
Layer 4: cgroups v2 resource limits                         ← cgroups.rs (exists)
Layer 5: Sigstore image verification                        ← NEW
Layer 6: Optional VM isolation (Cloud Hypervisor)           ← Future
```

```rust
// NEW FILE: crates/hyperbox-core/src/isolation/security_stack.rs

// SecurityStack composes all layers:
// 1. Read existing isolation modules
// 2. Create SecurityStack that orchestrates all layers
// 3. Each layer is optional (graceful degradation on older kernels)
// 4. Detect kernel capabilities at startup, enable maximum available protection
// 5. ZERO root privileges required for any layer
```

**Agent Assignment:** @CIPHER, @ARCHITECT  
**Priority:** P1  

#### 6C. Image Verification [REF:EVO-SC-603]

**Target:** `crates/hyperbox-core/src/storage/verification.rs` ← NEW FILE  

```rust
// 1. Add dependency: sigstore = "0.10" (sigstore-rs)  
// 2. Verify image signatures via Sigstore/Cosign
// 3. XGBoost-style risk scoring: combine Trivy scan metadata + package density
// 4. Block high-risk images by default, warn on medium
```

**Agent Assignment:** @CIPHER, @FORTRESS  
**Priority:** P2  

---

### EVOLUTION TRACK 7: DEVELOPER EXPERIENCE [REF:EVO-DX-700]

#### 7A. DevContainers Support [REF:EVO-DX-701]

**Target:** `crates/hyperbox-project/src/devcontainer.rs` ← NEW FILE  
**Why:** Docker removed Dev Environments from Desktop v4.42+. DevContainers (containers.dev) is the de facto standard, supported by VS Code, JetBrains, GitHub Codespaces. First-class support fills Docker's gap.

```rust
// NEW FILE: crates/hyperbox-project/src/devcontainer.rs

// 1. Parse .devcontainer/devcontainer.json
// 2. Support Features (OCI-based tool packages)
// 3. Support Templates  
// 4. Automatic port forwarding from devcontainer.json
// 5. Integration with existing ProjectManager
```

**Agent Assignment:** @APEX, @BRIDGE  
**Priority:** P1 — Fills Docker's abandoned gap  

#### 7B. Enhanced File Watcher (fanotify) [REF:EVO-DX-702]

**Target:** ENHANCE `crates/hyperbox-project/src/watcher.rs` (existing file)  
**Why:** Current implementation uses `notify` crate (inotify-based). fanotify provides mount-level monitoring superior to inotify for many files. Docker Compose Watch pattern: batching, debouncing, filtering.

```rust
// ENHANCE: crates/hyperbox-project/src/watcher.rs

// 1. Read existing watcher.rs first — preserve all current types
// 2. Add FanotifyWatcher as alternative backend  
// 3. Implement Docker Compose Watch modes:
//    - sync: Copy changed files into container
//    - rebuild: Trigger container rebuild on change
//    - sync+restart: Copy + restart container process
// 4. Batching + debouncing for filesystem change storms
// 5. Fall back to existing notify-based watcher when fanotify unavailable
```

**Agent Assignment:** @APEX  
**Priority:** P2  

#### 7C. P2P Image Sharing (Spegel) [REF:EVO-DX-703]

**Target:** `crates/hyperbox-optimize/src/p2p.rs` ← NEW FILE  
**Why:** Spegel (GA in K3s/RKE2, Dec 2024) is a stateless P2P OCI registry mirror. Each node serves images it already has. For teams: first developer pulls, teammates get instant serve from LAN.

```rust
// NEW FILE: crates/hyperbox-optimize/src/p2p.rs

// 1. Implement P2P image discovery via mDNS on local network
// 2. Serve locally cached images to peers
// 3. Route: Check local → Check P2P peers → Fall back to registry
// 4. Zero configuration required (stateless, auto-discovery)
```

**Agent Assignment:** @SYNAPSE, @STREAM  
**Priority:** P2  

#### 7D. Local Registry (zot) [REF:EVO-DX-704]

**Target:** `crates/hyperbox-core/src/storage/local_registry.rs` ← NEW FILE  
**Why:** zot (CNCF Sandbox) is a single statically-built binary, OCI-native storage with built-in deduplication and GraphQL search API. Replaces Docker's bespoke storage layer.

```rust
// NEW FILE: crates/hyperbox-core/src/storage/local_registry.rs

// 1. Embed zot as local image store
// 2. OCI-native storage (no Docker-specific format conversion)
// 3. Built-in deduplication at storage level
// 4. GraphQL API for image search/discovery
// 5. Multi-platform support (Linux, macOS, Windows via WSL)
```

**Agent Assignment:** @FORGE, @SYNAPSE  
**Priority:** P2  

---

### EVOLUTION TRACK 8: AI/ML FEATURES [REF:EVO-AI-800]

#### 8A. Local LLM Assistant [REF:EVO-AI-801]

**Target:** `crates/hyperbox-optimize/src/assistant.rs` ← NEW FILE  
**Why:** Docker's "Ask Gordon" (Feb 2025) demonstrates the pattern. HyperBox differentiator: Run locally via llama.cpp (llama-cpp-rs Rust bindings). 1-3B parameter model preserves privacy.

```rust
// NEW FILE: crates/hyperbox-optimize/src/assistant.rs

// 1. Add dependency: llama-cpp-rs = "latest" (or llama-cpp-2)
// 2. Implement LocalAssistant:
//    - Debug build errors from container logs
//    - Optimize Dockerfiles (suggest multi-stage, layer ordering)
//    - Suggest resource limits based on workload profile
//    - Natural language → CLI command translation
// 3. Model loading: Download 1-3B model on first use
// 4. Feature-gated: #[cfg(feature = "ai-assistant")]
// 5. NEVER require internet for inference (fully local)
```

**Agent Assignment:** @NEURAL, @LINGUA  
**Priority:** P3 — Differentiating feature, not blocking  

#### 8B. Smart Pre-Warming (Replace LSTM) [REF:EVO-AI-802]

**Target:** ENHANCE `crates/hyperbox-optimize/src/predict.rs` (existing file)  
**Why:** InstaInfer (SoCC 2024) showed ML artifact loading dominates cold start latency (68%). Heavy ML is overkill — simple frequency analysis with time-of-day/day-of-week patterns provides 90% of the value at 0% of the complexity.

```rust
// ENHANCE: crates/hyperbox-optimize/src/predict.rs

// 1. Read existing predict.rs — preserve all types
// 2. REPLACE LSTM complexity with SimplePredictor:
//    - Track container usage: (image, hour_of_day, day_of_week, frequency)
//    - Simple conditional probability: P(image | hour, dow)
//    - Pre-pull images when P > threshold (0.7)
//    - Zero external ML dependencies
// 3. Keep existing UsagePredictor interface, change internals
// 4. Add InstaInfer pattern: pre-pull model artifacts, not just images
```

**Agent Assignment:** @VELOCITY, @ORACLE  
**Priority:** P2  

---

### EVOLUTION TRACK 9: SUB-LINEAR DATA STRUCTURES [REF:EVO-SL-900]

#### 9A. Probabilistic Structures Module [REF:EVO-SL-901]

**Target:** `crates/hyperbox-optimize/src/structures.rs` ← NEW FILE  
**Why:** Sub-linear data structures provide O(1) operations where conventional structures require O(n).

```rust
// NEW FILE: crates/hyperbox-optimize/src/structures.rs

// 1. BloomFilter - Set membership O(1), 1.2MB for 1M items at 1% FPR
//    Use for: Dedup checks, "have we seen this chunk?"

// 2. CuckooFilter - Like Bloom but supports deletion, better at FPR <3%
//    Use for: Active port tracking, volume sets per container

// 3. HyperLogLog - Unique counting in 4KB for billions of items
//    Use for: Dashboard statistics (unique containers launched, connections served)

// 4. CountMinSketch - Frequency estimation in bounded space
//    Use for: Hot image tracking (which images used most frequently?)

// Dependencies: probabilistic-collections, hyperloglogplus
```

**Agent Assignment:** @VELOCITY, @AXIOM  
**Priority:** P2  

---

### EVOLUTION TRACK 10: DESKTOP APP ENHANCEMENTS [REF:EVO-UI-1000]

#### 10A. Real-Time Observability Dashboard [REF:EVO-UI-1001]

**Target:** ENHANCE `app/src/pages/Dashboard.tsx` and `app/src/pages/Performance.tsx`  
**Why:** Embed eBPF-powered observability directly in Tauri UI. Per-container flame graphs, network flow visibility, I/O latency tracking — all at under 2% CPU overhead.

```typescript
// ENHANCE existing Dashboard.tsx:
// 1. Read current file first
// 2. Add real-time metrics streaming via Tauri events
// 3. Recharts line/area charts for CPU, memory, network, I/O
// 4. Container-level drill-down
// 5. Project-level aggregate views
// 6. Memory pressure indicators (dynamic VM memory status)
```

**Agent Assignment:** @CANVAS, @SENTRY  
**Priority:** P1  

#### 10B. Container Terminal (xterm.js) [REF:EVO-UI-1002]

**Target:** `app/src/components/Terminal.tsx` ← NEW FILE  

```typescript
// 1. Add dependencies: xterm, xterm-addon-fit, xterm-addon-web-links
// 2. Full TTY support via Tauri command bridge
// 3. Multiple tabs for concurrent container shells
// 4. Search, copy, theming support
```

**Agent Assignment:** @CANVAS, @BRIDGE  
**Priority:** P1  

#### 10C. DevContainers UI [REF:EVO-UI-1003]

**Target:** `app/src/pages/DevEnvironments.tsx` ← NEW FILE  

```typescript
// 1. Browse and launch devcontainer.json configurations
// 2. Feature marketplace (browse OCI-based Features)
// 3. Template gallery for quick project setup
// 4. VS Code integration (one-click "Open in VS Code")
```

**Agent Assignment:** @CANVAS  
**Priority:** P2  

---

## 📦 DEPENDENCY ADDITIONS

### Add to `Cargo.toml` [workspace.dependencies] (APPEND, do not replace):

```toml
# === EVOLUTION v2.0 ADDITIONS ===

# Runtime — youki
# libcontainer = "0.5"  # Uncomment when youki crate stabilizes API

# Runtime — WASM
wasmtime = "latest"
wasmtime-wasi = "latest"

# Checkpoint/Restore
rust-criu = "0.4"

# Image Formats
# nydus-api = "latest"  # Check crates.io for current Nydus Rust crate names

# Deduplication  
fastcdc = "3"

# Security
landlock = "0.4"
sigstore = "0.10"

# Sub-linear structures
probabilistic-collections = "0.7"

# AI Assistant (optional)
# llama-cpp-2 = "latest"  # Feature-gated

# Persistent data structures
im = "15"  # Persistent/immutable collections (includes HAMT)
```

### CRITICAL: Version Resolution

Before adding any dependency:
1. Check `crates.io` for the ACTUAL latest version
2. Verify it compiles with our current Rust edition (2021)
3. Add behind feature flags when optional
4. Run `cargo check --workspace` after adding

---

## 🔄 EXECUTION PROTOCOL

### Phase Ordering (Execute Sequentially)

```
╔══════════════════════════════════════════════════════════════════╗
║  PHASE A: FOUNDATION HARDENING (Week 1-2)                       ║
║  Complete existing scaffolding into working implementations      ║
║                                                                  ║
║  A1. [EVO-RT-101] youki runtime integration                     ║
║  A2. [EVO-RS-201] Demand-paged CRIU restore                     ║
║  A3. [EVO-IF-301] Nydus lazy loading                            ║
║  A4. [EVO-SC-601] Landlock sandboxing                           ║
║  A5. [EVO-IF-302] composefs storage dedup                       ║
║                                                                  ║
║  GATE: `cargo test --workspace` passes                          ║
║  GATE: `cargo clippy --workspace -- -D warnings` passes         ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE B: COMPETITIVE ADVANTAGES (Week 3-4)                     ║
║  Strategic differentiators Docker cannot match                   ║
║                                                                  ║
║  B1. [EVO-RT-102] WASM runtime support                          ║
║  B2. [EVO-DX-701] DevContainers support                         ║
║  B3. [EVO-MR-501] Dynamic VM memory management                  ║
║  B4. [EVO-SC-602] Composable security stack                     ║
║  B5. [EVO-IF-303] FastCDC deduplication                         ║
║  B6. [EVO-UI-1001] Real-time dashboard                          ║
║  B7. [EVO-UI-1002] Container terminal                           ║
║  B8. [EVO-AI-802] Smart pre-warming                             ║
║                                                                  ║
║  GATE: Alpha-worthy functionality                               ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE C: ADVANCED FEATURES (Week 5-6)                          ║
║  Polish and advanced capabilities                                ║
║                                                                  ║
║  C1. [EVO-RT-103] Hybrid runtime selector                       ║
║  C2. [EVO-RS-202] Cooperative checkpointing                     ║
║  C3. [EVO-KA-401] netkit networking                             ║
║  C4. [EVO-KA-402] io_uring acceleration                         ║
║  C5. [EVO-MR-502] Persistent HAMT state                         ║
║  C6. [EVO-SC-603] Image verification                            ║
║  C7. [EVO-DX-702] Enhanced file watcher                         ║
║  C8. [EVO-DX-703] P2P image sharing                             ║
║  C9. [EVO-DX-704] Local registry (zot)                          ║
║  C10. [EVO-SL-901] Probabilistic structures                     ║
║  C11. [EVO-UI-1003] DevContainers UI                            ║
║  C12. [EVO-AI-801] Local LLM assistant                          ║
║                                                                  ║
║  GATE: Beta-worthy functionality                                ║
╚══════════════════════════════════════════════════════════════════╝
```

### Per-Task Execution Checklist

For EVERY task, follow this exact sequence:

```
□ 1. Read ALL files you intend to modify (completely, not just headers)
□ 2. Read the module's mod.rs to understand current exports
□ 3. Read the crate's Cargo.toml to understand current dependencies
□ 4. Plan changes as ADDITIVE (new files preferred, minimal existing file edits)
□ 5. If adding a dependency, run: cargo add <dep> -p <crate> (or edit Cargo.toml)
□ 6. Run: cargo check -p <affected-crate>
□ 7. Fix any compilation errors before proceeding
□ 8. Write unit tests for new code
□ 9. Run: cargo test -p <affected-crate>
□ 10. Run: cargo clippy -p <affected-crate> -- -D warnings
□ 11. If all pass: git add . && git commit -m "feat(<crate>): <description>"
□ 12. If ANY step fails: fix before moving to next task
```

### Autonomous Decision Rules

```
┌──────────────────────────────────────────────────────────────────┐
│  AUTONOMOUS: Proceed without asking                              │
│  ─────────────────────────────────────────────────────────────   │
│  • Creating NEW files in any crate                              │
│  • Adding new dependencies behind feature flags                 │
│  • Adding new public methods to existing structs                │
│  • Adding new trait implementations                             │
│  • Creating feature branches                                    │
│  • Writing tests                                                │
│  • Fixing compilation errors you introduced                     │
│  • Adding cfg feature gates                                     │
│                                                                  │
│  PAUSE AND ASK: Require confirmation                            │
│  ─────────────────────────────────────────────────────────────   │
│  • Changing existing public trait signatures                    │
│  • Removing or renaming existing public types                   │
│  • Modifying existing test expectations                         │
│  • Changing workspace-level Cargo.toml structure                │
│  • Anything that would break `cargo check --workspace`          │
│  • Changing the Tauri config (tauri.conf.json)                  │
│  • Modifying CI/CD workflows                                    │
└──────────────────────────────────────────────────────────────────┘
```

---

## 🎯 PERFORMANCE TARGETS (Validated by Research)

| Metric | Docker Desktop | HyperBox Target | Mechanism |
|--------|---------------|-----------------|-----------|
| OCI cold start | 225ms | <50ms | youki library integration |
| WASM cold start | N/A (deprecated) | <1ms | Wasmtime AOT compilation |
| Warm restore | N/A | <10ms | Demand-paged CRIU + hot pages |
| Image pull (cached) | 5-30s | Near-zero | Nydus lazy loading + P2P |
| Storage per duplicate image | 100% | ~3% | composefs + FastCDC dedup |
| Networking overhead | ~35% | ~0% | netkit (kernel 6.8+) |
| Idle RAM | 300-500MB | <40MB | Dynamic VM memory + Tauri |
| Installer size | 600MB | <15MB | Rust/Tauri native |
| Security setup | Root required | Zero root | Landlock + user namespaces |

---

## 🤖 AGENT ASSIGNMENTS (Evolution Phase)

| Track | Tasks | Primary Agent | Support Agents |
|-------|-------|--------------|----------------|
| Runtime | EVO-RT-101, 102, 103 | @APEX | @CORE, @NEXUS, @ARCHITECT |
| Instant Restore | EVO-RS-201, 202 | @VELOCITY | @CORE |
| Image Formats | EVO-IF-301, 302, 303 | @VELOCITY | @FORGE, @AXIOM |
| Kernel Acceleration | EVO-KA-401, 402 | @CORE | @VELOCITY, @CIPHER |
| Memory/Resources | EVO-MR-501, 502 | @VELOCITY | @CORE, @AXIOM |
| Security | EVO-SC-601, 602, 603 | @CIPHER | @FORTRESS, @ARCHITECT |
| Developer Experience | EVO-DX-701, 702, 703, 704 | @APEX | @BRIDGE, @SYNAPSE, @STREAM, @FORGE |
| AI/ML | EVO-AI-801, 802 | @NEURAL | @LINGUA, @VELOCITY, @ORACLE |
| Sub-Linear | EVO-SL-901 | @VELOCITY | @AXIOM |
| Desktop UI | EVO-UI-1001, 1002, 1003 | @CANVAS | @SENTRY, @BRIDGE |

---

## 📋 QUICK-START COMMAND

When you receive this prompt, begin immediately with Phase A, Task A1:

```
1. Open S:\HyperBox in your workspace
2. Read crates/hyperbox-core/src/runtime/mod.rs
3. Read crates/hyperbox-core/src/runtime/traits.rs  
4. Read crates/hyperbox-core/src/runtime/crun.rs (reference implementation)
5. Create crates/hyperbox-core/src/runtime/youki.rs
6. Implement YoukiRuntime for the existing ContainerRuntime trait
7. Register in mod.rs behind #[cfg(feature = "youki")]
8. Run cargo check -p hyperbox-core --features youki
9. Write tests
10. Commit and proceed to A2
```

**@OMNISCIENT EVOLUTION PROTOCOL: ACTIVATED**  
**Elite Agent Collective Status: PHASE 2 DEPLOYED**  
**Mode: INTEGRITY-FIRST ADDITIVE ENHANCEMENT**  
**Target: Complete 20× Docker Desktop Replacement**  

---

*Evolve with the precision of @AXIOM, the speed of @VELOCITY, the security of @CIPHER, and the vision of @GENESIS.*

**🧬 HYPERBOX EVOLUTION INITIATED 🧬**
