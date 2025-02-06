# Code Execution System Improvements

## Critical Issues

### Sandbox

1. Process spawn failing with "Invalid argument (os error 22)"
   - Root cause: Incorrect resource limits or sandbox setup
   - Affects all language executions
2. Resource limits not enforced
   - Timeout assertions failing
   - Memory/CPU limits not triggering
3. Output capture broken
   - Stdout/stderr handling issues
   - Process termination unreliable

### Language Support

1. Python
   - Syntax check failing
   - Virtualenv setup issues
   - Dependency installation noisy
2. JavaScript/TypeScript
   - npm install errors flooding output
   - Type definitions missing
   - Module resolution failing
3. Go
   - Module initialization noise
   - Dependency resolution failing
4. Rust
   - Toolchain setup messages in tests
   - Cargo output noise

### Test Infrastructure

1. Logging noise
   - Package manager output flooding tests
   - Compilation messages cluttering output
   - Virtual environment creation spam
2. Test reliability
   - Inconsistent process isolation
   - Resource limit tests flaky
   - Poor cleanup between tests

## Action Items

### Phase 1: Core (Week 1)

1. Fix process spawn errors
   ```rust
   // Sandbox changes needed:
   - Validate resource limits before apply
   - Proper error handling for rlimit
   - Fix sandbox directory permissions
   ```
2. Implement proper process isolation
3. Fix resource limiting
4. Add quiet mode for all operations

### Phase 2: Languages (Week 1-2)

1. Implement logging capture for package managers
2. Add silent installation modes
3. Fix environment setup issues
4. Proper cleanup between tests

### Phase 3: Testing (Week 2)

1. Add test isolation
2. Implement proper resource limit tests
3. Add logging control
4. Improve error reporting

## Implementation Notes

### Sandbox Changes

```rust
pub struct Sandbox {
    root_dir: PathBuf,
    limits: ResourceLimits,
    logger: Logger,
}

impl Sandbox {
    fn validate_limits(&self) -> Result<()> {
        // Validate resource limits before applying
        // Check for valid ranges
        // Verify system capabilities
    }

    fn setup_environment(&self) -> Result<()> {
        // Set proper permissions
        // Configure process limits
        // Setup logging capture
    }

    async fn execute_process(&self, cmd: &str) -> Result<Output> {
        // Real process execution
        // Proper resource limiting
        // Reliable timeout enforcement
    }
}
```
