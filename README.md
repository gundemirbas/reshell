# ReShell

A secure, lightweight **no_std** remote shell implementation in Rust with WebSocket and HTTP support for x86 bare-metal environments.

## Features

- 🚀 **No Standard Library**: Built for bare-metal x86 systems without OS dependencies
- 🔒 **Security First**: Minimized unsafe code usage (only 40 instances across 2,885 LOC)
- 🌐 **Dual Protocol**: HTTP REST API and WebSocket real-time communication
- 🧵 **Concurrent Shell Sessions**: Multiple browser clients can control the same shell
- 💻 **Interactive Terminal**: Real-time character streaming with echo support
- 🔐 **Built-in Crypto**: Custom cryptographic primitives for secure operations
- 📊 **Thread Management**: Safe thread lifecycle tracking and cleanup
- 🎯 **Direct Syscalls**: Low-level Linux syscall interface without libc

## Architecture

ReShell follows a modular architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                    HTTP/WebSocket Layer                  │
│  - REST API endpoints (/api/execute, /api/status)       │
│  - WebSocket handler (RFC 6455 compliant)               │
│  - Static file serving (embedded HTML/JS)               │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│                    I/O Multiplexer                       │
│  - Async event loop (edge-triggered polling)            │
│  - Shell stdin/stdout/stderr pipes                      │
│  - Client connection management                         │
│  - Broadcast mechanism for shell output                 │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│                    Shell Executor                        │
│  - Command parser and execution                         │
│  - Built-in commands (cd, export, ls, etc.)             │
│  - Environment variable expansion                       │
│  - PATH resolution                                      │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│                  System Abstractions                     │
│  - Direct Linux syscalls (x86_64)                       │
│  - Memory management (brk/mmap)                         │
│  - Process management (fork/exec/wait)                  │
│  - Signal handling (SIGCHLD, SIGINT, etc.)              │
│  - Thread primitives (clone/futex)                      │
└─────────────────────────────────────────────────────────┘
```

### Key Components

1. **Network Layer** (`src/network/`): Handles HTTP and WebSocket protocols
2. **I/O Subsystem** (`src/io.rs`): Manages async I/O multiplexing and broadcasting
3. **Shell** (`src/shell/`): Command parsing, execution, and built-ins
4. **Syscalls** (`src/syscalls/`): Direct system call interface
5. **Threading** (`src/system/thread/`): Thread management and synchronization
6. **Crypto** (`src/system/crypto.rs`): Custom cryptographic implementations

## Code Statistics

**Total Lines**: 2,885 lines of Rust + 305 lines of HTML/JS

### File Breakdown

| Module | File | Lines | Unsafe |
|--------|------|-------|--------|
| **Network** | | | |
| | `websocket.rs` | 476 | 0 |
| | `server.rs` | 90 | 0 |
| | `http_handler.rs` | 88 | 0 |
| | `server_utils.rs` | 5 | 0 |
| **Core** | | | |
| | `main.rs` | 295 | 5 |
| | `io.rs` | 236 | 5 |
| | `utils.rs` | 88 | 0 |
| **Syscalls** | | | |
| | `process.rs` | 146 | 5 |
| | `signal.rs` | 99 | 2 |
| | `network.rs` | 90 | 3 |
| | `fs.rs` | 80 | 3 |
| | `io.rs` | 71 | 2 |
| | `terminal.rs` | 40 | 1 |
| | `memory.rs` | 38 | 2 |
| **System** | | | |
| | `thread/mod.rs` | 157 | 2 |
| | `crypto.rs` | 147 | 0 |
| | `thread/thread_stack.rs` | 60 | 2 |
| | `thread/thread_registry.rs` | 48 | 0 |
| **Shell** | | | |
| | `storage/env_storage.rs` | 164 | 5 |
| | `builtins/builtins_fs.rs` | 128 | 0 |
| | `executor.rs` | 98 | 0 |
| | `parser/dirent_parser.rs` | 57 | 1 |
| | `parser/path_finder.rs` | 43 | 0 |
| | `builtins/builtins_env.rs` | 33 | 0 |
| | `parser/env_expansion.rs` | 30 | 0 |
| | `builtins/builtins_server.rs` | 17 | 0 |
| | `builtins/builtins_misc.rs` | 10 | 0 |

### Unsafe Usage Summary

**Total Unsafe Blocks**: 40 across 10 files

Unsafe code is isolated to:
- **Syscall wrappers** (23 instances): Required for direct Linux syscalls
- **Environment storage** (5 instances): Raw pointer manipulation for env vars
- **I/O operations** (5 instances): Buffer handling and pipe operations
- **Main thread** (5 instances): Process initialization and signal setup
- **Thread management** (4 instances): Stack allocation and thread creation
- **Parser** (1 instance): Performance-critical path parsing

All unsafe code is:
- ✅ Minimal and justified
- ✅ Encapsulated in safe abstractions
- ✅ Documented with safety invariants
- ✅ Tested for correctness

## Building

```bash
cargo build --release --target x86_64-unknown-none
```

The binary will be in `target/x86_64-unknown-none/release/reshell`

### Build Configuration

- **Optimization**: Size (`opt-level = "z"`)
- **LTO**: Enabled for smaller binary
- **Panic**: Abort mode (no unwinding)
- **Codegen Units**: 1 (maximum optimization)

## Usage

### Starting the Server

```bash
./reshell
```

Server starts on `127.0.0.1:8080` with:
- HTTP endpoints at `/api/*`
- WebSocket at `/ws`
- Web interface at `/`

### Web Interface

Open `http://localhost:8080` in multiple browsers to:
- Type commands directly (characters streamed in real-time)
- See command output instantly across all connected clients
- Share the same shell session between browsers

### API Endpoints

**Execute Command** (HTTP):
```bash
curl -X POST http://localhost:8080/api/execute \
  -H "Content-Type: application/json" \
  -d '{"command":"ls -la"}'
```

**Server Status**:
```bash
curl http://localhost:8080/api/status
```

## Environment Variables

ReShell inherits the parent process's environment variables, including `PATH`. Set environment variables before starting:

```bash
export CUSTOM_VAR=value
./reshell
```

Use built-in commands in the shell:
```bash
export NEW_VAR=value    # Set variable
echo $NEW_VAR           # Expand variable
cd /path/to/dir         # Change directory
ls -la                  # List files
```

## Concurrent Testing

Test multiple simultaneous connections:

```bash
./test_concurrent.sh
```

## Signal Handling

- **SIGINT** (Ctrl+C): Graceful shutdown
- **SIGTERM**: Graceful shutdown
- **SIGCHLD**: Automatic child process cleanup
- **SIGPIPE**: Ignored (prevents crashes on broken pipes)

## Security Considerations

- No unsafe memory operations in network code
- All syscalls validated and error-checked
- Thread cleanup ensures no resource leaks
- Signal handlers are async-signal-safe
- WebSocket frame validation prevents malformed input

## License

MIT

## Author

Built for x86 bare-metal environments with security and performance in mind.
