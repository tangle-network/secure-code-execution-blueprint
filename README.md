# <h1 align="center">Code Execution Blueprint for Tangle Network 🚀</h1>

## 📚 Overview

This Tangle Blueprint provides a secure code execution service that runs arbitrary code snippets in a Trusted Execution Environment (TEE). It supports multiple programming languages and ensures secure isolation through sandboxing and resource limits.

The service is designed to be:

- 🔒 Secure: Runs code in isolated environments with strict resource limits
- 🌐 Language-agnostic: Supports multiple programming languages
- ⚡ Fast: Optimized for quick code execution and response
- 🛡️ Safe: Leverages TEE for secure code execution
- 🔄 Scalable: Handles concurrent executions with proper resource management

### 💫 Key Capabilities

| Capability                | Description                                               |
| ------------------------- | --------------------------------------------------------- |
| **Execution Isolation**   | Each code snippet runs in its own sandboxed environment   |
| **Resource Control**      | Fine-grained control over memory, CPU, and disk usage     |
| **Concurrent Processing** | Handle multiple code executions simultaneously            |
| **Security Measures**     | TEE protection, resource limits, and process isolation    |
| **Language Support**      | Easy integration of new programming languages             |
| **Monitoring**            | Real-time tracking of resource usage and execution status |

## 📋 Prerequisites

Before running this project, ensure you have:

- [Rust](https://www.rust-lang.org/tools/install)
- [Forge](https://getfoundry.sh)
- [cargo-tangle](https://crates.io/crates/cargo-tangle)

Install cargo-tangle:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tangle-network/gadget/releases/download/cargo-tangle-v0.1.2/cargo-tangle-installer.sh | sh
```

Or via crates.io:

```bash
cargo install cargo-tangle --force
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

## 🔒 Security

The service implements multiple security measures:

- Sandboxed execution environment
- Resource limits and monitoring
- Process isolation
- Secure cleanup after execution
- Input validation and sanitization

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## 🤝 Contributing

We welcome contributions! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
