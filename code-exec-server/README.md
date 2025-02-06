# Code Execution Server

A secure HTTP server for executing code in various programming languages.

## Features

- Supports multiple programming languages (Python, JavaScript, TypeScript, Go, Rust)
- Resource limiting (memory, CPU, disk space)
- Concurrent execution support
- Sandbox environment for secure execution
- RESTful API
- Docker support

## API Endpoints

### Health Check

```
GET /health
```

Returns 200 OK if the server is running.

### Execute Code

```
POST /execute
Content-Type: application/json

{
  "language": "python",
  "code": "print('Hello, World!')",
  "input": null,
  "dependencies": [],
  "timeout": 5000,
  "env_vars": {}
}
```

## Running Locally

1. Build and run directly:

```bash
cargo run --release
```

2. Using Docker:

```bash
docker-compose up --build
```

## Configuration

The server can be configured using command-line arguments:

```bash
code-exec-server --help
```

Available options:

- `--addr`: Server address (default: 0.0.0.0:3000)
- `--max-concurrent`: Maximum concurrent executions (default: 10)
- `--memory-limit`: Memory limit in bytes (default: 100MB)
- `--cpu-time-limit`: CPU time limit in seconds (default: 5)
- `--max-processes`: Maximum number of processes (default: 10)
- `--file-size-limit`: File size limit in bytes (default: 10MB)
- `--disk-space-limit`: Disk space limit in bytes (default: 100MB)

## Docker Deployment

The server can be deployed using Docker:

1. Build the image:

```bash
docker build -t code-exec-server -f code-exec-server/Dockerfile .
```

2. Run the container:

```bash
docker run -p 3000:3000 \
  --security-opt seccomp=unconfined \
  --cap-add SYS_ADMIN \
  -v /tmp:/tmp \
  code-exec-server
```

Or using docker-compose:

```bash
docker-compose up --build
```

## Security Considerations

The server uses several security measures:

- Sandbox environment for code execution
- Resource limits (memory, CPU, disk space)
- Process isolation
- Non-root user in Docker
- Temporary directories for execution

## Development

1. Run tests:

```bash
cargo test
```

2. Format code:

```bash
cargo fmt
```

3. Run linter:

```bash
cargo clippy
```
