# PTY Screen Interception & Automation in a Custom Terminal Emulator

A comprehensive guide to intercepting, analyzing, and automating interactions within a PTY-based terminal emulator (Alacritty + Rust), with a focus on detecting prompts from CLI tools (e.g., Claude CLI) and programmatically responding to them.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Reading: Interception Methods](#2-reading-interception-methods)
   - 2.1 [Raw PTY Stream Tee (Master FD)](#21-raw-pty-stream-tee-master-fd)
   - 2.2 [VT State Parsing (Virtual Terminal)](#22-vt-state-parsing-virtual-terminal)
   - 2.3 [Screen Buffer Snapshots](#23-screen-buffer-snapshots)
   - 2.4 [Custom Escape Sequences (OSC/DCS)](#24-custom-escape-sequences-oscdcs)
   - 2.5 [Pattern Matching on Stripped Text](#25-pattern-matching-on-stripped-text)
   - 2.6 [Process State & Signal Monitoring](#26-process-state--signal-monitoring)
   - 2.7 [TIOCPKT (Packet Mode)](#27-tiocpkt-packet-mode)
   - 2.8 [Idle Detection via poll/epoll](#28-idle-detection-via-pollepoll)
   - 2.9 [ptrace / strace on Child](#29-ptrace--strace-on-child)
   - 2.10 [eBPF / Kernel Tracepoints](#210-ebpf--kernel-tracepoints)
   - 2.11 [Script / Terminal Recording](#211-script--terminal-recording)
   - 2.12 [Accessibility APIs](#212-accessibility-apis)
   - 2.13 [IPC Channel (Unix Socket)](#213-ipc-channel-unix-socket)
3. [Writing: Injecting Input into the PTY](#3-writing-injecting-input-into-the-pty)
4. [Recommended Architecture for Full Automation](#4-recommended-architecture-for-full-automation)
5. [Implementation Skeleton (Rust)](#5-implementation-skeleton-rust)
6. [Relevant Crates & Libraries](#6-relevant-crates--libraries)
7. [Comparison Matrix](#7-comparison-matrix)

---

## 1. Architecture Overview

A PTY (pseudoterminal) consists of a master/slave pair. The child process (e.g., a shell or Claude CLI) is attached to the slave side. The terminal emulator holds the master side. All data flows through the master fd:

```
┌─────────────────────────────────────────────────────────────────┐
│                        YOUR APPLICATION                         │
│                                                                 │
│  ┌──────────┐    ┌───────────┐    ┌────────────┐               │
│  │ Alacritty │◄───│   Proxy   │◄───│  PTY       │               │
│  │ Renderer  │    │  (tee +   │    │  Master FD │               │
│  │           │    │  analyze) │───►│            │               │
│  └──────────┘    └───────────┘    └─────┬──────┘               │
│                        │                │                       │
│                  ┌─────▼──────┐         │ kernel                │
│                  │ VT Parser  │         │                       │
│                  │ (shadow    │    ┌────▼──────┐               │
│                  │  screen)   │    │ PTY Slave  │               │
│                  └─────┬──────┘    └────┬──────┘               │
│                        │               │                       │
│                  ┌─────▼──────┐   ┌────▼──────┐               │
│                  │ Automation │   │ Child      │               │
│                  │ Engine     │   │ Process    │               │
│                  │ (pattern   │   │ (claude,   │               │
│                  │  match +   │   │  bash...)  │               │
│                  │  respond)  │   └────────────┘               │
│                  └────────────┘                                 │
└─────────────────────────────────────────────────────────────────┘

Data flow:
  Child writes to slave  ──►  appears on master fd (read)
  Write to master fd     ──►  appears as keyboard input on slave (child reads it)
```

The key insight: **the master fd is bidirectional**. Reading from it gives you the child's output; writing to it simulates keyboard input to the child.

---

## 2. Reading: Interception Methods

### 2.1 Raw PTY Stream Tee (Master FD)

**Concept:** Read from the master fd in a dedicated thread/task and duplicate (tee) the byte stream — one copy goes to the renderer, the other to your analyzer.

**How it works:**
```rust
// Pseudocode
loop {
    let n = read(master_fd, &mut buf)?;
    let data = &buf[..n];

    // Fork the stream
    renderer_tx.send(data.to_vec())?;   // to Alacritty for display
    analyzer_tx.send(data.to_vec())?;   // to your analysis pipeline
}
```

**Pros:**
- Zero-copy awareness of everything the child outputs
- No external dependencies or kernel features needed
- Works on all POSIX systems

**Cons:**
- Raw bytes include ANSI escape sequences — not human-readable without parsing
- Must handle partial reads and UTF-8 boundary splits

---

### 2.2 VT State Parsing (Virtual Terminal)

**Concept:** Feed the raw byte stream into a VT100/xterm parser to maintain a shadow screen buffer that mirrors what the user sees.

**Options:**

| Crate | Description |
|---|---|
| `vte` | Low-level VT parser (same one Alacritty uses). Emits events for printable chars, escape sequences, CSI params, etc. You build the screen state yourself. |
| `alacritty_terminal` | Full terminal emulation library. Provides `Term` struct with a grid of cells. Feed bytes in, read screen content out. |
| `termwiz` (from wezterm) | Complete VT parser + screen buffer. `Surface` / `Terminal` types give you cell-level access. |

**Using `alacritty_terminal` as a headless shadow terminal:**
```rust
use alacritty_terminal::term::Term;
use alacritty_terminal::event::EventListener;

// Create a shadow Term with same dimensions as the visible terminal
let mut shadow_term = Term::new(config, dimensions, event_proxy);

// Feed raw PTY output into it
for byte in raw_bytes {
    shadow_term.advance(byte);
}

// Read current screen content
for line in shadow_term.grid().display_iter() {
    // extract text from cells
}
```

**Pros:**
- Accurate representation of what's on screen (handles cursor movement, scrolling, alternate screen, colors, etc.)
- Can query specific screen regions

**Cons:**
- Must keep the shadow terminal in perfect sync with the real one
- Some overhead for maintaining a second terminal state

---

### 2.3 Screen Buffer Snapshots

**Concept:** If you control the terminal emulator (forked Alacritty or using `alacritty_terminal` as a lib), directly access the internal grid/screen buffer.

**How it works:**
- `Term::grid()` returns the cell grid
- Iterate rows and columns to extract text content
- Trigger snapshots on events (idle, new data, timer)

```rust
fn snapshot_screen(term: &Term) -> String {
    let mut text = String::new();
    let grid = term.grid();
    for row in grid.display_iter() {
        for cell in row {
            text.push(cell.c);
        }
        text.push('\n');
    }
    text
}
```

**Pros:**
- Exact representation of user-visible content
- No separate parsing needed — reuses the real terminal state

**Cons:**
- Tight coupling to Alacritty internals
- Requires access to the `Term` instance (threading/locking considerations)

---

### 2.4 Custom Escape Sequences (OSC/DCS)

**Concept:** If you control the child process (or can wrap it), emit custom escape sequences that carry structured metadata about state transitions.

**Protocol example:**
```
# Child sends:
\x1b]777;state;waiting_for_input\x07
\x1b]777;state;generating\x07
\x1b]777;prompt;Do you want to proceed? [y/N]\x07

# Terminal intercepts OSC 777 and routes to automation engine
```

**Existing standards:**
- **OSC 133** — Shell Integration / Semantic Prompts (used by iTerm2, WezTerm, VS Code terminal). Marks prompt start, command start, command end, output end.
- **OSC 7** — Current working directory notification
- **OSC 9** — Desktop notifications (ConEmu/Kitty)

**Pros:**
- Clean, structured, unambiguous signaling
- No heuristics or pattern matching needed
- Can carry arbitrary metadata

**Cons:**
- Requires cooperation from the child process
- If you don't control the CLI tool, this isn't an option (unless you write a wrapper)

---

### 2.5 Pattern Matching on Stripped Text

**Concept:** Strip ANSI escape codes from the raw stream, then apply regex or string matching to detect prompts, questions, errors, etc.

```rust
use strip_ansi_escapes::strip;

let clean = strip(&raw_bytes);
let text = String::from_utf8_lossy(&clean);

// Detect common patterns
let patterns = [
    r"(?i)\[y/n\]",
    r"(?i)do you want to (proceed|continue)",
    r"(?i)press enter to",
    r"\?\s*$",  // line ending with ?
    r"(?i)(error|failed|abort)",
];

for pattern in &patterns {
    if Regex::new(pattern).unwrap().is_match(&text) {
        // Trigger automation
    }
}
```

**Pros:**
- Simple to implement
- Works without any VT parsing for basic cases
- Good for quick prototyping

**Cons:**
- Fragile — breaks when output format changes
- False positives on partial escape sequences or wrapped lines
- Cannot understand screen layout (only stream order)

---

### 2.6 Process State & Signal Monitoring

**Concept:** Monitor the child process to detect when it's waiting for input vs. actively producing output.

**Methods:**

| Technique | What it reveals |
|---|---|
| `/proc/<pid>/status` → `State: S` | Process is sleeping (likely waiting for I/O) |
| `/proc/<pid>/wchan` | Kernel function the process is blocked on (e.g., `wait_woken` for tty read) |
| `/proc/<pid>/syscall` | Current syscall number and arguments — detect `read(0, ...)` |
| `waitpid(WNOHANG)` | Check if child exited or was stopped |
| `SIGCHLD` handler | Get notified when child stops/exits |

**Detecting "waiting for input":**
```rust
fn is_waiting_for_stdin(pid: u32) -> bool {
    let wchan = std::fs::read_to_string(format!("/proc/{}/wchan", pid))
        .unwrap_or_default();
    // Common wchan values when blocked on tty read:
    wchan.contains("wait_woken")
        || wchan.contains("n_tty_read")
        || wchan.contains("do_select")
}
```

**Pros:**
- Non-invasive (reads /proc, no ptrace needed)
- Can definitively tell if the child is blocked waiting for input

**Cons:**
- Linux-specific (/proc filesystem)
- Polling-based (not event-driven)
- Doesn't tell you *what* the prompt says, only that input is expected

---

### 2.7 TIOCPKT (Packet Mode)

**Concept:** Enable packet mode on the PTY master fd. The kernel then prefixes every `read()` result with a status byte containing flags about PTY state changes.

```rust
use libc::{ioctl, TIOCPKT};

let on: libc::c_int = 1;
unsafe { ioctl(master_fd, TIOCPKT, &on) };

// Now every read() returns: [status_byte] [data...]
let n = read(master_fd, &mut buf)?;
let status = buf[0];
let data = &buf[1..n];

if status & libc::TIOCPKT_DATA != 0 {
    // Normal data follows
}
if status & libc::TIOCPKT_STOP != 0 {
    // Output stopped (child called tcflow TCOOFF or similar)
}
if status & libc::TIOCPKT_START != 0 {
    // Output restarted
}
if status & libc::TIOCPKT_FLUSHREAD != 0 {
    // Read queue flushed
}
if status & libc::TIOCPKT_FLUSHWRITE != 0 {
    // Write queue flushed
}
```

**Flags:**

| Flag | Meaning |
|---|---|
| `TIOCPKT_DATA` | Normal data (the rest of the buffer is output bytes) |
| `TIOCPKT_FLUSHREAD` | Read side of PTY was flushed |
| `TIOCPKT_FLUSHWRITE` | Write side of PTY was flushed |
| `TIOCPKT_STOP` | Output to terminal was stopped |
| `TIOCPKT_START` | Output to terminal was restarted |
| `TIOCPKT_NOSTOP` | Stop/start characters are no longer being used |
| `TIOCPKT_DOSTOP` | Stop/start characters are being used again |

**Pros:**
- Kernel-level event notification — very efficient
- Catches flow control events you can't see otherwise

**Cons:**
- Doesn't reveal *content* — only PTY state transitions
- Must be enabled before the child is spawned (or at least before reads begin)
- Adds complexity to the read loop (must strip status byte)

---

### 2.8 Idle Detection via poll/epoll

**Concept:** Monitor the master fd for read readiness. When no new data arrives for N milliseconds, the screen has "stabilized" — this is the ideal moment to take a snapshot and analyze content.

```rust
use nix::poll::{poll, PollFd, PollFlags};

let mut pollfd = PollFd::new(master_fd, PollFlags::POLLIN);
let timeout_ms = 100; // 100ms idle threshold

loop {
    match poll(&mut [pollfd], timeout_ms)? {
        0 => {
            // Timeout: no data for 100ms — screen is stable
            let screen = snapshot_screen(&shadow_term);
            check_for_prompts(&screen);
        }
        _ => {
            // Data available — read and forward
            let n = read(master_fd, &mut buf)?;
            forward_to_renderer(&buf[..n]);
            feed_to_shadow_term(&buf[..n]);
        }
    }
}
```

**Pros:**
- Natural debouncing — avoids analyzing mid-render
- Efficient (kernel-level waiting, no busy polling)
- Pairs perfectly with screen snapshots

**Cons:**
- The "right" timeout value is heuristic (too short = false triggers during slow output; too long = sluggish response)
- Doesn't distinguish between "child is thinking" and "child is waiting for input"

---

### 2.9 ptrace / strace on Child

**Concept:** Attach to the child process with `ptrace` and intercept syscalls to observe `read(0, ...)` (waiting for stdin) and `write(1, ...)` / `write(2, ...)` (producing output).

```rust
use nix::sys::ptrace;
use nix::sys::wait::waitpid;

ptrace::attach(child_pid)?;
loop {
    waitpid(child_pid, None)?;
    let regs = ptrace::getregs(child_pid)?;

    // On x86_64: rax=0 is read, rdi=0 is fd 0 (stdin)
    if regs.orig_rax == 0 && regs.rdi == 0 {
        // Child is calling read(stdin) — waiting for input!
        notify_automation_engine();
    }

    ptrace::syscall(child_pid, None)?;
}
```

**Pros:**
- Definitive detection of "child wants input"
- Can also observe what the child writes (full I/O interception)

**Cons:**
- Significant performance overhead (every syscall is trapped)
- Invasive — may interfere with child process behavior
- Architecture-specific register access
- Cannot be combined with other debuggers
- Read-only — cannot inject input through ptrace

---

### 2.10 eBPF / Kernel Tracepoints

**Concept:** Attach eBPF programs to kernel tracepoints or kprobes related to TTY I/O to observe PTY traffic without modifying either the child or the terminal.

**Relevant attach points:**
- `tracepoint/syscalls/sys_enter_read` — filter by fd pointing to the PTY slave
- `tracepoint/syscalls/sys_enter_write` — catch child output
- `kprobe/tty_write` — direct TTY layer hook
- `kprobe/n_tty_read` — line discipline read

**Tools:** `libbpf-rs`, `aya` (Rust eBPF framework), `bpftrace` for prototyping.

**Pros:**
- Zero modification to child or terminal
- Minimal performance impact (JIT-compiled in kernel)
- Can filter by specific PTY device number

**Cons:**
- Requires root/CAP_BPF
- Complex to set up
- Read-only — cannot inject input
- Kernel version dependent

---

### 2.11 Script / Terminal Recording

**Concept:** Use the classic `script(1)` approach — wrap the PTY so all I/O is recorded to a file or pipe.

**Variants:**
- `script -q /path/to/typescript` — classic recording
- **asciicast** format (asciinema) — structured JSON with timestamps
- Custom Rust implementation: tee all master fd reads/writes to a log file

**Pros:**
- Simple, well-understood
- Good for debugging and replay

**Cons:**
- Passive recording — not real-time analysis
- Requires post-processing for automation
- Better suited for debugging than live automation

---

### 2.12 Accessibility APIs

**Concept:** Use OS-level accessibility APIs to read terminal screen content.

| Platform | API |
|---|---|
| Linux | AT-SPI2 (D-Bus based) |
| macOS | NSAccessibility / AXUIElement |
| Windows | UI Automation / MSAA |

**Pros:**
- Non-invasive, works with unmodified terminals
- Structured access to text content

**Cons:**
- Alacritty has limited accessibility support currently
- High latency compared to direct fd access
- Read-only — no standard way to inject input
- Platform-specific

---

### 2.13 IPC Channel (Unix Socket)

**Concept:** Add a Unix domain socket or named pipe to your terminal emulator that exposes an API for querying screen state and injecting input.

**API example:**
```json
// Request
{"method": "get_screen_text", "params": {"rows": [0, 24]}}

// Response
{"text": "$ claude\nHello! How can I help?\n> ", "cursor": {"row": 2, "col": 2}}

// Send input
{"method": "send_keys", "params": {"text": "yes\n"}}
```

**Pros:**
- Clean separation of concerns
- Can be used by external tools (scripts, other programs)
- Bidirectional

**Cons:**
- Requires modifying the terminal emulator
- Must design and maintain the IPC protocol
- Additional complexity (serialization, error handling, authentication)

---

## 3. Writing: Injecting Input into the PTY

There is fundamentally **one mechanism** for sending input to a PTY child process:

```rust
// Writing to the master fd = simulating keyboard input
nix::unistd::write(master_fd, b"y\n")?;
```

This is equivalent to the user physically typing `y` and pressing Enter. The child process sees it as normal stdin input.

**Variations:**

```rust
// Send a single character
write(master_fd, b"y")?;

// Send text + Enter
write(master_fd, b"hello world\n")?;

// Send Ctrl+C (interrupt)
write(master_fd, &[0x03])?;

// Send Ctrl+D (EOF)
write(master_fd, &[0x04])?;

// Send Ctrl+Z (suspend)
write(master_fd, &[0x1a])?;

// Send escape key
write(master_fd, &[0x1b])?;

// Send arrow keys (escape sequences)
write(master_fd, b"\x1b[A")?;  // Up
write(master_fd, b"\x1b[B")?;  // Down
write(master_fd, b"\x1b[C")?;  // Right
write(master_fd, b"\x1b[D")?;  // Left

// Send Tab (for autocompletion)
write(master_fd, b"\t")?;
```

**Important considerations:**
- Write atomically when possible (single `write()` call for a complete response) to avoid race conditions with child reads.
- Respect the child's terminal mode (raw vs. cooked). In cooked mode, the line discipline buffers until `\n`. In raw mode, each byte is delivered immediately.
- Be careful with timing — writing too fast after detecting a prompt might race with the child setting up its read.

---

## 4. Recommended Architecture for Full Automation

For a complete system that can detect Claude CLI prompts and automatically respond:

```
┌─────────────────────────────────────────────────────────────────────┐
│                          MAIN EVENT LOOP                            │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    PTY PROXY LAYER                           │   │
│  │                                                              │   │
│  │  1. poll(master_fd, timeout=100ms)                           │   │
│  │  2. On data: read() → tee to renderer + shadow_term          │   │
│  │  3. On timeout (idle): trigger analysis                      │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                              │                                      │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │                   ANALYSIS LAYER                             │   │
│  │                                                              │   │
│  │  1. Shadow Term (alacritty_terminal) → screen text           │   │
│  │  2. /proc/<pid>/wchan → is child blocked on read?            │   │
│  │  3. Pattern matching on screen text                          │   │
│  │  4. Combine signals: idle + waiting_for_input + prompt       │   │
│  │     detected = HIGH CONFIDENCE prompt                        │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                              │                                      │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │                   AUTOMATION LAYER                           │   │
│  │                                                              │   │
│  │  match detected_state {                                      │   │
│  │    Prompt::YesNo(q)     => write(master_fd, b"y\n"),         │   │
│  │    Prompt::FreeText(q)  => write(master_fd, response),       │   │
│  │    Prompt::Error(e)     => notify_user(e),                   │   │
│  │    State::Generating    => /* wait */,                       │   │
│  │    State::Finished      => /* next task */,                  │   │
│  │  }                                                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

**Confidence scoring for prompt detection:**

| Signal | Weight | Source |
|---|---|---|
| Screen text matches prompt pattern | High | VT parser + regex |
| Child process blocked on `read(0)` | High | /proc/pid/wchan |
| No new data for >100ms | Medium | poll timeout |
| TIOCPKT_STOP received | Low | packet mode |
| Last line ends with `?` or `>` | Medium | text analysis |

When multiple high-confidence signals align, trigger the automation response.

---

## 5. Implementation Skeleton (Rust)

```rust
use std::os::unix::io::{AsRawFd, RawFd};
use std::io::{Read, Write};
use nix::pty::openpty;
use nix::unistd::{fork, ForkResult, dup2, execvp, read as nix_read, write as nix_write};
use nix::poll::{poll, PollFd, PollFlags};
use nix::sys::signal::Signal;

struct PtyProxy {
    master_fd: RawFd,
    child_pid: u32,
    // shadow_term: Term,  // from alacritty_terminal
    idle_timeout_ms: i32,
}

impl PtyProxy {
    fn run(&mut self) {
        let mut buf = [0u8; 8192];
        let pollfd = PollFd::new(self.master_fd, PollFlags::POLLIN);

        loop {
            match poll(&mut [pollfd], self.idle_timeout_ms).unwrap() {
                0 => {
                    // Idle — screen is stable, analyze
                    self.on_idle();
                }
                _ => {
                    // Data available
                    let n = nix_read(self.master_fd, &mut buf).unwrap();
                    if n == 0 { break; } // EOF — child exited

                    let data = &buf[..n];

                    // 1. Forward to renderer
                    self.forward_to_renderer(data);

                    // 2. Feed to shadow terminal
                    self.feed_shadow_term(data);
                }
            }
        }
    }

    fn on_idle(&mut self) {
        // 1. Snapshot screen text
        let screen_text = self.get_screen_text();

        // 2. Check if child is waiting for input
        let waiting = self.is_child_waiting_for_input();

        // 3. Pattern match
        if waiting {
            if let Some(action) = self.detect_prompt(&screen_text) {
                self.execute_action(action);
            }
        }
    }

    fn detect_prompt(&self, screen: &str) -> Option<Action> {
        // Yes/No prompts
        if screen.contains("[y/N]") || screen.contains("[Y/n]") {
            return Some(Action::SendText("y\n".into()));
        }

        // "Do you want to proceed?"
        if screen.to_lowercase().contains("do you want to proceed") {
            return Some(Action::SendText("yes\n".into()));
        }

        // Waiting for free-text input (e.g., Claude prompt)
        if screen.ends_with("> ") || screen.ends_with(">>> ") {
            return Some(Action::SendFromQueue);
        }

        // Error detection
        if screen.to_lowercase().contains("error") {
            return Some(Action::NotifyUser);
        }

        None
    }

    fn execute_action(&self, action: Action) {
        match action {
            Action::SendText(text) => {
                nix_write(self.master_fd, text.as_bytes()).unwrap();
            }
            Action::SendFromQueue => {
                // Pull next message from automation queue
                if let Some(msg) = self.dequeue_message() {
                    nix_write(self.master_fd, msg.as_bytes()).unwrap();
                    nix_write(self.master_fd, b"\n").unwrap();
                }
            }
            Action::NotifyUser => {
                // Signal to UI or log
            }
        }
    }

    fn is_child_waiting_for_input(&self) -> bool {
        let wchan = std::fs::read_to_string(
            format!("/proc/{}/wchan", self.child_pid)
        ).unwrap_or_default();
        wchan.contains("wait_woken")
            || wchan.contains("n_tty_read")
            || wchan.contains("poll_schedule_timeout")
    }

    fn get_screen_text(&self) -> String {
        // Extract text from shadow_term grid
        // Implementation depends on terminal library used
        todo!()
    }

    fn forward_to_renderer(&self, data: &[u8]) {
        // Send to Alacritty rendering pipeline
        todo!()
    }

    fn feed_shadow_term(&mut self, data: &[u8]) {
        // Feed bytes into shadow Term
        todo!()
    }

    fn dequeue_message(&self) -> Option<String> {
        // Get next automation message from queue
        todo!()
    }
}

enum Action {
    SendText(String),
    SendFromQueue,
    NotifyUser,
}
```

---

## 6. Relevant Crates & Libraries

### Core PTY & Terminal

| Crate | Purpose |
|---|---|
| `nix` | POSIX APIs (pty, ioctl, poll, signals, fork/exec) |
| `libc` | Raw libc bindings (TIOCPKT, ioctl constants) |
| `portable-pty` | Cross-platform PTY abstraction (from wezterm) |
| `openpty` / `nix::pty` | PTY creation |

### VT Parsing & Terminal Emulation

| Crate | Purpose |
|---|---|
| `vte` | Low-level VT parser (Alacritty's parser) |
| `alacritty_terminal` | Full terminal emulation with grid/cell access |
| `termwiz` | VT parsing + surface buffer (from wezterm) |

### Text Processing

| Crate | Purpose |
|---|---|
| `strip-ansi-escapes` | Remove ANSI escape codes from byte streams |
| `regex` | Pattern matching on cleaned text |
| `aho-corasick` | Multi-pattern string matching (fast) |

### Expect-like Automation

| Crate | Purpose |
|---|---|
| `rexpect` | Expect-style PTY automation (spawn + wait + send) |
| `expectrl` | Modern expect library for Rust |

### eBPF (Advanced)

| Crate | Purpose |
|---|---|
| `aya` | Pure-Rust eBPF framework |
| `libbpf-rs` | libbpf bindings for Rust |

---

## 7. Comparison Matrix

| Method | Read | Write | Accuracy | Complexity | Invasiveness | Platform |
|---|---|---|---|---|---|---|
| Master FD tee | ✅ | ✅ | Raw bytes | Low | None | POSIX |
| VT parser (shadow term) | ✅ | ❌ | Screen-level | Medium | None | Any |
| Screen buffer snapshot | ✅ | ❌ | Exact | Low | Terminal mod | Any |
| Custom OSC sequences | ✅ | ❌ | Structured | Low | Child mod | Any |
| Pattern matching | ✅ | ❌ | Heuristic | Low | None | Any |
| /proc state monitoring | ✅ | ❌ | Process-level | Low | None | Linux |
| TIOCPKT | ✅ | ❌ | Flow control | Medium | None | POSIX |
| poll/epoll idle | ✅ | ❌ | Timing-based | Low | None | POSIX |
| ptrace | ✅ | ❌ | Syscall-level | High | High | Linux |
| eBPF | ✅ | ❌ | Kernel-level | High | None (root) | Linux 4.x+ |
| IPC socket | ✅ | ✅ | Custom API | Medium | Terminal mod | Any |
| **write(master_fd)** | ❌ | **✅** | **Direct** | **Trivial** | **None** | **POSIX** |

**Key takeaway:** For bidirectional automation, the optimal stack is:

1. **Master FD tee** (read the stream)
2. **VT parser / shadow term** (understand the screen)
3. **Idle detection** (know when to analyze)
4. **Pattern matching** (detect prompts)
5. **`/proc` state check** (confirm child wants input)
6. **`write(master_fd)`** (send the response)
