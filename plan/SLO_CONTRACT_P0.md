# SLO Contract & Measurement Plan
## Phase 0 Deliverable
**Date**: 2026-04-27  
**Status**: APPROVED  
**Owner**: Mathi

---

## 1. SLO METRICS (Hard Gates for v1 Release)

### 1.1 Startup Time
**SLO**: Startup time <1000ms (p50: <500ms)

**Measurement**:
```
Method: Measure cold start (fresh kill, no cache) vs warm start (DB populated)
Tool: trace system calls (Windows: Event Tracing; macOS: Instruments)
Protocol:
  1. Kill all Mathi processes
  2. Clear temp cache (if any)
  3. Run `time mathi` (wall-clock elapsed)
  4. Record timestamp from process fork → first UI frame visible
  5. Repeat 10 times; report p50, p95, max
```

**Acceptance**:
- Cold start: <1000ms (95th percentile)
- Warm start: <500ms (95th percentile)
- Max observed: <2000ms (p99)

**Owner**: P6 instrumentation lead  
**Gate**: Release blocked if p95 > 1000ms

---

### 1.2 Editor Typing Latency
**SLO**: Typing latency <5ms (p95, keystroke → screen update)

**Measurement**:
```
Method: Measure keystroke latency under active agent load
Tool: frame profiler (60fps baseline; measure delta between keystroke and pixel change)
Scenario:
  1. Open a 10KB source file
  2. Start 2 agents running in background (Code Fix swarm, idle waiting)
  3. Type 100 keystrokes sequentially; measure p95 latency per keystroke
  4. Repeat 3 times; average the results
  5. Measure with and without memory pressure (spawn agents until idle RAM = 150MB)
```

**Acceptance**:
- Baseline (no agents): <3ms (p95)
- With agents active: <5ms (p95)
- Max observed: <10ms (p99)

**Owner**: P2 editor lead  
**Gate**: Release blocked if p95 > 5ms with agents active

---

### 1.3 Idle Memory
**SLO**: Idle memory <150MB (measured 1h after startup, no active swarms)

**Measurement**:
```
Method: Resident Set Size (RSS) capture
Tool: OS memory profiler (Windows: Task Manager or /proc/self/status; macOS: Activity Monitor)
Protocol:
  1. Start Mathi; load a moderate workspace (50 files, 2MB total)
  2. Wait 1 hour (no agent interactions, just idle editor)
  3. Snapshot memory every 5min for the last 15min
  4. Exclude OS page cache and graphics memory
  5. Report mean + stdev of final 15min snapshot
```

**Acceptance**:
- Idle RSS: <150MB (mean over final 15min)
- Max observed in idle period: <200MB (p99)

**Exclusions**: OS page cache, GPU memory, memory-mapped files not in RSS.

**Owner**: P1 runtime lead  
**Gate**: Release blocked if mean idle > 150MB

---

### 1.4 Agent Time-to-First-Token (TTFT)
**SLO**: <100ms from task dispatch → first streaming token received

**Measurement**:
```
Method: Measure end-to-end latency across 3 primary adapters
Adapters:
  1. Gemini CLI (via `gemini-cli --stream`)
  2. OpenCode (VS Code Copilot CLI adapter)
  3. Claude Code (Anthropic API adapter)

Protocol per adapter:
  1. Prepare a 1KB code snippet + simple prompt ("fix syntax errors")
  2. Dispatch task to agent pool
  3. Record: time.now() → first byte of streaming response
  4. Repeat 10 times; measure p50, p95, max
  5. Repeat 3 times (control for cold vs warm JIT)

Aggregate:
  1. Report p95 TTFT per adapter
  2. Report worst-case adapter p95
```

**Acceptance**:
- Per-adapter p95: <100ms
- Worst-case p95: <150ms (up to 50ms adapter variation allowed)
- Max observed: <300ms (p99)

**Owner**: P3 agent platform lead  
**Gate**: Release blocked if worst-case p95 > 150ms

---

### 1.5 Swarm Task Handoff Overhead
**SLO**: <50ms from leader dispatch → worker ack (excluding adapter execution)

**Measurement**:
```
Method: IPC timing under swarm load
Protocol:
  1. Start a 4-worker swarm (Code Fix template)
  2. Dispatch 100 tasks in rapid succession; each task is a no-op (just ack)
  3. Measure latency for each task: send → ack received at leader
  4. Report p50, p95, max of the 100 samples
  5. Repeat under 80% worker utilization (3/4 workers busy) and 100% utilization
```

**Acceptance**:
- Baseline (low load): <30ms (p95)
- High load (80% util): <50ms (p95)
- Max observed: <100ms (p99)

**Owner**: P1 orchestrator lead  
**Gate**: Release blocked if p95 > 50ms under any load

---

### 1.6 No Memory Leaks in Long-Running Swarms
**SLO**: Idle RSS stable over 24h under simulated continuous load

**Measurement**:
```
Method: Memory growth monitoring
Protocol:
  1. Start Mathi; spawn a swarm (Refactor template, 3 agents)
  2. Continuously spawn tasks (1 task/5sec) for 24 hours
  3. Each task: agent analyzes a 1KB file, returns synthetic result (simulated work)
  4. Snapshot memory every 30min
  5. Compute linear regression on memory samples; report slope (bytes/hour)
  6. Repeat twice to validate
```

**Acceptance**:
- Memory growth slope: <1MB/hour (within noise)
- Max observed RSS during test: <400MB (150MB idle + 250MB headroom)
- No OOM kills over 24h

**Owner**: P5 memory system lead  
**Gate**: E2E test in P6; must pass before release

---

### 1.7 Graceful Degradation Under Load
**SLO**: 10+ concurrent tasks, all collected, no data loss, cancellation is 100% reliable

**Measurement**:
```
Method: Stress test under concurrent load
Protocol:
  1. Start Mathi; spawn large swarm (Research template, 6 agents)
  2. Spawn 20 concurrent tasks (exceed typical worker pool capacity)
  3. Measure: memory, task success rate, cancellation latency
  4. Scenario A: Let all tasks complete normally
  5. Scenario B: User cancels session mid-flight; verify all tasks cleaned up (no orphans)
  6. Verify no data corruption in result cache or approval store
```

**Acceptance**:
- Task success rate: 100% (no dropped tasks)
- Cancellation: 100% of in-flight tasks cancelled within 2s
- Memory during peak load: <500MB (headroom included)
- No orphaned processes after cancel

**Owner**: P4 sandbox/cleanup lead  
**Gate**: E2E test in P6; must pass before release

---

## 2. MEASUREMENT INFRASTRUCTURE (P1/P2)

### 2.1 Telemetry Points
```rust
// Core measurement hooks (add during P1)
pub fn record_startup_time(duration_ms: u64);
pub fn record_keystroke_latency(latency_ms: u64);
pub fn record_memory_snapshot(rss_bytes: u64);
pub fn record_ttft(adapter: &str, latency_ms: u64);
pub fn record_handoff_latency(latency_ms: u64);
pub fn record_task_result(status: TaskStatus, duration_ms: u64);
```

### 2.2 Data Collection
- **Local storage**: SQLite table `telemetry_samples` (not synced)
- **UI dashboard**: Show recent 100 samples; rolling histogram
- **Export**: CSV dump for analysis; optional redaction of payload data

### 2.3 Failure Detection
```rust
// If any gate violated, trigger alert:
if ttft_p95 > 150ms {
    log_violation("TTFT_GATE_VIOLATED", ttft_p95);
    ui_alert("Agent response time degraded; check network/adapter health");
}
```

---

## 3. ACCEPTANCE CRITERIA (P6 Gate)

**Release Criteria** (all must pass):
- [ ] Startup time p95 < 1000ms (cold)
- [ ] Typing latency p95 < 5ms (with agents)
- [ ] Idle memory mean < 150MB (after 1h)
- [ ] Agent TTFT p95 < 150ms (worst-case adapter)
- [ ] Task handoff p95 < 50ms (high load)
- [ ] 24h memory test: slope < 1MB/hour
- [ ] Stress test: 10+ tasks, 100% success, 100% cancellation
- [ ] No critical bugs identified in E2E smoke test suite

**Failure Recovery**:
- If any gate violated: mark as `RELEASE_BLOCKED`, escalate to Mathi
- Diagnosis: Run targeted profiler (perf, flamegraph, heapdump)
- Retarget: Identify phase to address (e.g., memory leak → P5)

---

## 4. DEPENDENCY CHECK (P0 Deliverable)

**Required Tools**:
- Rust 1.70+ (check: `rustc --version`) ✅ 1.95.0
- Cargo (check: `cargo --version`) ✅ 1.95.0
- Tauri CLI 2.0+ (check: `cargo install tauri-cli --version ^2.0`) — Install if missing
- SQLite 3.40+ (check: `sqlite3 --version`) — Usually pre-installed on modern OS

**Command to verify all**:
```bash
rustc --version && cargo --version && cargo tauri --version && sqlite3 --version
```

---

## 5. SLO VERSIONING

| Version | Date | Changes |
|---------|------|---------|
| v1.0 | 2026-04-27 | Initial SLO contract locked for P0. |
| | | Acceptance gates defined; measurement harness outlined. |

---

## Sign-Off

**Approved By**: Mathi  
**Date**: 2026-04-27  
**Status**: Locked for v1 execution
