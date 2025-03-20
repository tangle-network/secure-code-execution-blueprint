# <h1 align="center">Code Execution Blueprint for Tangle Network 🚀</h1>

## 📚 Overview

This Tangle Blueprint provides a secure code execution service that runs arbitrary code snippets in a Trusted Execution Environment (TEE). It supports multiple programming languages and ensures secure isolation through sandboxing and resource limits.

The service is designed to be:

- 🔒 Secure: Runs code in isolated environments with strict resource limits
- 🌐 Language-agnostic: Supports multiple programming languages
- ⚡ Fast: Optimized for quick code execution and response
- 🛡️ Safe: Leverages TEE for secure code execution
- 🔄 Scalable: Handles concurrent executions with proper resource management

## 📋 Prerequisites

Before running this project, ensure you have:

- [Rust](https://www.rust-lang.org/tools/install)
- [Forge](https://getfoundry.sh)
- [cargo-tangle](https://crates.io/crates/cargo-tangle)

Install cargo-tangle:

```bash
cargo install cargo-tangle --git https://github.com/tangle-network/blueprint.git --force
```

## 🚀 Quick Start

1. **Build the Project**:

```bash
cargo build
```

2. **Run Tests**:

```bash
cargo test
```

3. **Deploy the Blueprint**:

```bash
cargo tangle blueprint deploy
```

## 💻 Usage

### Execute Code via HTTP API

```bash
curl -X POST http://localhost:8080/execute \
  -H "Content-Type: application/json" \
  -d '{
    "language": "python",
    "code": "print(\"Hello, World!\")",
    "input": null,
    "timeout": 30
  }'
```

Response format:

```json
{
  "stdout": "Hello, World!\n",
  "stderr": "",
  "status": "success",
  "execution_time": 123,
  "memory_usage": 1024
}
```

### Execute Code via Tangle Network

```rust
let result = execute_code(
    "python".to_string(),
    "print('Hello from Tangle!')",
    None,
    context
).await?;
```

## 🔧 Configuration

The service can be configured through environment variables:

- `CODE_EXEC_PORT`: HTTP server port (default: 8080)
- `MAX_CONCURRENT_EXECUTIONS`: Maximum concurrent code executions (default: 10)

Resource limits can be customized in `ResourceLimits`:

```rust
ResourceLimits {
    memory: 256 * 1024 * 1024,  // 256MB
    cpu_time: 30,               // 30 seconds
    processes: 32,              // Max 32 processes
    file_size: 10 * 1024 * 1024, // 10MB
    disk_space: 100 * 1024 * 1024, // 100MB
}
```

## 🏗️ Architecture

The blueprint consists of several key components:

1. **CodeExecutionService**: Core service managing code execution
2. **Sandbox**: Isolated environment for secure code execution
3. **Language Executors**: Language-specific execution implementations
4. **HTTP Server**: RESTful API for code execution requests
5. **Resource Monitor**: Tracks and limits resource usage

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
