## Methodologies
### Implementation Methodology
When presented with a request YOU MUST:
1. Use context7 mcp server or websearch tool to get the latest related documentation. Understand the API deeply and all of its nuances and options
2. Use TDD Approach: Figure out how to validate that the task is complete and working as expected. Whether using a CLI tool like curl, or ssh command or writing unit/integration test
3. Start with the simplest happy path test. The test should fail on `unimplemented!()|raise NotImplemented| throw Error("Not Implmented")`. Scaffold out all functions (including full signature) with this body 
4. See the test fail with not implemented error.
5. Make the smallest change possible
6. Take time to think through the most optimal order of operations for implementation
7. Check if tests and `(npm |cargo cmd | uv run) checks` passes
8. Repeat step 5-6 until the test passes
9. You MUST NOT move on until tests pass

### Debugging Methodology

#### Phase I: Information Gathering

1. Understand the error
2. Read the relevant source code: try local `.venv`, `node_modules` or `$HOME/.cargo/registry/src/`
3. Look at any relevant github issues for the library

#### Phase II: Testing Hypothesis
4. Develop a hypothesis that resolves the root cause of the problem. Must only chase root cause possible solutions.  Think hard to decide if its root cause or NOT.
5. Add debug logs to determine hypothesis
6. If not successful, YOU MUST clean up any artifact or code attempts in this debug cycle. Then repeat steps 1-5

#### Phase III: Weigh Tradeoffs
7. If successful and fix is straightforward. Apply fix
8. If not straightforward, weigh the tradeoffs and provide a recommendation



## 🧱 Code Structure & Modularity
- **Follow SOLID Principles***
- **Never Break Up nested Values:** When working with a value that is part of a larger
  structure or has a parent object, always import or pass the entire parent structure
  as an argument. Never extract or isolate the nested value from its parent context.
- **Write Elegant Code** Write the most minimal code to get the job done
- **Get to root of the problem** Never write hacky workarounds. You are done when the tests pass 
- **Never create a file longer than 200 lines of code.** If a file approaches this limit, refactor by splitting it into modules or helper files.
- **Use cfg.yml file for config variable. You MUST NOT add config vars to env files.**
- **Use template-secrets.env file to keep track of the list of secrets:**
- **Use environment variables for secrets** Do NOT conflate secrets with config variables
- **Keep it generic class/type names: TimeseriesClient instead of TimeScaleClient**
- **Use Generics Judiciously:** Remember, while generics are powerful, they can also make code more complex if
  overused. Always consider readability and maintainability when deciding whether to
  use generics. If the use of generics doesn't provide a clear benefit in terms of
  code reuse, type safety, or API design, it might be better to use concrete types
  instead.

### 🧪 Testing & Reliability
- **Fail fast, fail early**: Detect errors as early as possible and halt execution. Rely on the runtime or system to handle the error and provide a stack trace.  You MUST NOT write random error handling for no good reason.
- **Unit Tests should be colocated in `src/`**
- **Integration Tests** should be located in `tests/`
- **Use AAA (Arrange, Act, Assert) pattern for tests**:
  - **Arrange**: Set up the necessary context and inputs.
  - **Act**: Execute the code under test.
  - **Assert**: Verify the outcome matches expectations.
- **Use testcontainers for integration tests** — spin up real databases/services in Docker, session-scoped for performance

## 💅 Style
- **Constants in code:** Write top level declarations in SCREAMING_SNAKE_CASE.

### 📚 Documentation & Explainability
- **Comment non-obvious code** and ensure everything is understandable to a mid-level developer.
- When writing complex logic, **add an inline `Reason:` comment** explaining the why, not just the what.
- **Write concise document comments for primarily for an LLM to consume, secondarily for a document generator to consume**


## Rust Language Guidelines 🦀

### Rust Patterns
- **Prefer pattern matching:** Use `match` for comprehensive control flow
- **Prefer validated types:** Use custom types with the [validator](https://docs.rs/validator/0.15.0/validator/)
- **Prefer functional programming:** Use iterators with `map`/`filter`/`fold` for transforming collections
- **Use `format!` macro** for string formatting
- **Use `const` for compile-time constants** with SCREAMING_SNAKE_CASE

### Rust Testing Guidelines
- **Use actual/expected semantics:** `assert_eq!(actual, expected);`

### Error Handling
- **Propagate errors with `?` operator** - let them bubble up
- **Use `Box<dyn Error>` for simple error handling**

### Memory & Ownership

- **Prefer borrowing to cloning:** Use `&T` instead of `T.clone()` when possible
- **Use `String` for owned strings** and `&str` for borrowed string slices

### Common Macros & Attributes

- **Use `#[allow(dead_code)]`** sparingly and document why

### Commenting

- **Write concise RustDoc comments for an llm to consume:**

```rust
/// Calculates basic statistics for numeric data.
///
/// # Arguments
/// * `data` - Slice of f64 values, filters out NaN/infinite
///
/// # Panics
/// Panics if data is empty or contains no valid numbers
///
/// # Example
/// ```rust
/// let result = calculate_stats(&[1.0, 2.0, 3.0]);
/// assert_eq!(result.mean, 2.0);
/// ```
pub fn calculate_stats(data: &[f64]) -> Stats {
```


## Rust Embassy Guidelines 📟

### Client-Driver Pattern
- **Use client-driver patterns** established in the `temperature/` example. Use it as a reference when creating new resources.
- **Driver:** Low-level hardware interaction (sensor reads, register access)
- **Client:** High-level API that wraps the driver (unit conversion, data formatting)

### Embassy Async Runtime
- **Use `#[embassy_executor::main]`** as the entry point
- **Use `embassy_time::Timer`** for async delays instead of blocking sleep
- **Spawn tasks with `spawner.spawn()`** for concurrent operations

### Hardware Abstraction
- **Use ESP-HAL peripherals** — initialize via `esp_hal::init()` with config
- **Configure pins explicitly** — use typed GPIO pins for safety
- **Use feature flags** for conditional compilation (e.g., `defmt`, `esp-hal`, `integration-test`)

### WiFi & MQTT
- **Initialize WiFi with `esp_wifi`** — connect using credentials from `cfg.yml`
- **Use async MQTT client** for publishing sensor data
- **Handle reconnection logic** for unreliable network environments

### Testing Strategy
- **Unit tests:** `cargo test --lib` on host target (no hardware needed)
- **Integration tests:** `embedded-test` crate with testcontainers for service dependencies
- **Hardware-in-loop tests:** `hil_test` running against real device with `--test-threads=1`


