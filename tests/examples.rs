//! Integration tests for WASI examples
//!
//! These tests build and run the example servers, making HTTP requests to verify
//! they work correctly.
//!
//! Run with: `cargo test --test examples`
//! Run specific test: `cargo test --test examples test_http`
//! Run with output: `cargo test --test examples -- --nocapture`

use std::collections::{HashMap, HashSet};
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

// Constants for timeouts and intervals
const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const SERVER_READINESS_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const SERVER_READINESS_DELAY: Duration = Duration::from_millis(500);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_millis(100);

/// Configuration for testing an example
///
/// This struct defines all the parameters needed to test an example:
/// - How to build it (always required)
/// - How to run it (optional for build-only tests)
/// - What HTTP endpoint to test (method, path, body)
/// - Any external dependencies (environment variables, Docker services)
#[derive(Debug, Clone)]
struct ExampleConfig {
    /// Example name (e.g., "http", "blobstore")
    name: &'static str,
    /// HTTP method to use for testing
    method: HttpMethod,
    /// Path to test (e.g., "/", "/health", "/cache")
    path: &'static str,
    /// JSON body to send (if any)
    body: Option<&'static str>,
    /// Required environment variables (skip test if missing)
    required_env: &'static [&'static str],
    /// Whether this example needs Docker services
    needs_docker: bool,
    /// Docker compose file (relative to repo root)
    docker_compose: Option<&'static str>,
    /// Whether to only test building (skip runtime test)
    build_only: bool,
}

/// HTTP methods supported for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    /// Get the HTTP method as a string
    const fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
        }
    }
}

impl ExampleConfig {
    const fn new(name: &'static str) -> Self {
        Self {
            name,
            method: HttpMethod::Post,
            path: "/",
            body: Some(r#"{"text":"hello"}"#),
            required_env: &[],
            needs_docker: false,
            docker_compose: None,
            build_only: false,
        }
    }

    const fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    const fn path(mut self, path: &'static str) -> Self {
        self.path = path;
        self
    }

    const fn body(mut self, body: Option<&'static str>) -> Self {
        self.body = body;
        self
    }

    const fn required_env(mut self, vars: &'static [&'static str]) -> Self {
        self.required_env = vars;
        self
    }

    const fn needs_docker(mut self, compose_file: &'static str) -> Self {
        self.needs_docker = true;
        self.docker_compose = Some(compose_file);
        self
    }

    const fn build_only(mut self) -> Self {
        self.build_only = true;
        self
    }
}

/// All example configurations (computed once and cached)
static EXAMPLE_CONFIGS: LazyLock<HashMap<&'static str, ExampleConfig>> = LazyLock::new(|| {
    let mut configs = HashMap::new();

    // Standalone examples - no Docker needed
    configs.insert("blobstore", ExampleConfig::new("blobstore"));
    configs.insert("http", ExampleConfig::new("http"));
    configs.insert("keyvalue", ExampleConfig::new("keyvalue"));
    configs.insert("otel", ExampleConfig::new("otel"));
    configs.insert("vault", ExampleConfig::new("vault"));

    // Standalone examples - need specific routes or env vars
    configs.insert(
        "http-proxy",
        ExampleConfig::new("http-proxy").method(HttpMethod::Get).path("/cache").body(None),
    );
    configs.insert(
        "identity",
        ExampleConfig::new("identity")
            .method(HttpMethod::Get)
            .body(None)
            .required_env(&["IDENTITY_TOKEN_URL"]),
    );
    configs.insert(
        "websockets",
        ExampleConfig::new("websockets").method(HttpMethod::Get).path("/health").body(None),
    );

    // Build-only examples (need external services for runtime)
    configs.insert("messaging", ExampleConfig::new("messaging").build_only());
    configs.insert("sql", ExampleConfig::new("sql").build_only());

    // Docker-dependent examples
    configs.insert(
        "blobstore-mongodb",
        ExampleConfig::new("blobstore-mongodb").needs_docker("docker/mongodb.yaml"),
    );
    configs.insert(
        "blobstore-nats",
        ExampleConfig::new("blobstore-nats").needs_docker("docker/nats.yaml"),
    );
    configs.insert(
        "keyvalue-nats",
        ExampleConfig::new("keyvalue-nats").needs_docker("docker/nats.yaml"),
    );
    configs.insert(
        "keyvalue-redis",
        ExampleConfig::new("keyvalue-redis").needs_docker("docker/redis.yaml"),
    );
    configs.insert(
        "messaging-kafka",
        ExampleConfig::new("messaging-kafka").needs_docker("docker/kafka.yaml").path("/pub-sub"),
    );
    configs.insert(
        "messaging-nats",
        ExampleConfig::new("messaging-nats")
            .needs_docker("docker/nats.yaml")
            .path("/request-reply"),
    );
    configs.insert(
        "sql-postgres",
        ExampleConfig::new("sql-postgres")
            .needs_docker("docker/postgres.yaml")
            .method(HttpMethod::Get)
            .body(None),
    );
    configs.insert(
        "vault-azure",
        ExampleConfig::new("vault-azure").needs_docker("docker/azurekv.yaml"),
    );

    configs
});

/// Get example configurations (cached, computed once at first access)
///
/// This function returns a reference to a static HashMap that is initialized
/// lazily on first use. Subsequent calls return the same cached reference.
fn get_example_configs() -> &'static HashMap<&'static str, ExampleConfig> {
    &EXAMPLE_CONFIGS
}

/// Get the repository root directory
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Find a free port to use for testing
///
/// This binds to port 0, which tells the OS to assign any available port,
/// then immediately releases it. There's a small race condition where another
/// process could claim the port before we use it, but this is unlikely in practice.
fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to random port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

/// Check if required environment variables are set
fn check_required_env(vars: &[&str]) -> Result<(), Vec<String>> {
    let missing: Vec<String> =
        vars.iter().filter(|&var| env::var(var).is_err()).map(|&s| s.to_string()).collect();

    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

/// Check if Docker compose services are running
fn docker_service_running(compose_file: &str) -> bool {
    let compose_path = repo_root().join(compose_file);
    if !compose_path.exists() {
        eprintln!("Docker compose file not found: {}", compose_path.display());
        return false;
    }

    let Some(compose_path_str) = compose_path.to_str() else {
        eprintln!("Invalid compose file path (non-UTF8): {}", compose_path.display());
        return false;
    };

    let Ok(output) = Command::new("docker")
        .args(["compose", "-f", compose_path_str, "ps", "--status", "running"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    else {
        eprintln!("Failed to execute docker compose command");
        return false;
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Docker compose check failed: {stderr}");
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Check if there are any running containers (more than just the header)
    let running_count = stdout.lines().filter(|line| !line.trim().is_empty()).count();
    running_count > 1
}

/// Track which examples have been built to avoid rebuilding them
///
/// This is a thread-safe cache of example names that have already been built.
/// Multiple tests can run in parallel and share this cache to avoid redundant builds.
static BUILT_EXAMPLES: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Build a specific example (WASM guest and host)
///
/// This function ensures an example is built by:
/// 1. Checking if it's already in the build cache
/// 2. Building the WASM guest component (wasm32-wasip2 target)
/// 3. Building the host binary
/// 4. Adding it to the build cache on success
///
/// The build cache is thread-safe, so multiple tests can call this concurrently.
fn ensure_example_built(example: &str) -> Result<(), String> {
    // Check if already built
    if BUILT_EXAMPLES.lock().expect("Failed to lock BUILT_EXAMPLES").contains(example) {
        return Ok(());
    }

    eprintln!("Building example: {example}...");

    // Build WASM guest
    let wasm_example = format!("{example}-wasm");
    let wasm_status = Command::new("cargo")
        .current_dir(repo_root())
        .args([
            "build",
            "--example",
            &wasm_example,
            "--target",
            "wasm32-wasip2",
            "--message-format=short",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run WASM build for {example}: {e}"))?;

    if !wasm_status.success() {
        return Err(format!("WASM build failed for {example} with status: {wasm_status}"));
    }

    // Build host
    let host_status = Command::new("cargo")
        .current_dir(repo_root())
        .args(["build", "--example", example, "--message-format=short"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run host build for {example}: {e}"))?;

    if !host_status.success() {
        return Err(format!("Host build failed for {example} with status: {host_status}"));
    }

    // Mark as built
    BUILT_EXAMPLES.lock().expect("Failed to lock BUILT_EXAMPLES").insert(example.to_string());

    Ok(())
}

/// Server handle that kills the process on drop and manages output threads
struct ServerHandle {
    child: Child,
    name: String,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl ServerHandle {
    fn new(
        child: Child, name: &str, stdout_thread: Option<JoinHandle<()>>,
        stderr_thread: Option<JoinHandle<()>>,
    ) -> Self {
        Self {
            child,
            name: name.to_string(),
            stdout_thread,
            stderr_thread,
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        eprintln!("Stopping server: {}", self.name);

        // Kill the process first
        if let Err(e) = self.child.kill() {
            eprintln!("Warning: Failed to kill server {}: {e}", self.name);
        }

        // Wait for the process
        if let Err(e) = self.child.wait() {
            eprintln!("Warning: Failed to wait for server {}: {e}", self.name);
        }

        // Join the output threads (they should finish once the process exits)
        if let Some(thread) = self.stdout_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.stderr_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Helper function to spawn a thread to capture and log output
///
/// This creates a background thread that reads from the given stream line-by-line
/// and logs each line with a prefix. The thread will exit when the stream is closed.
fn logger(
    stream: impl Read + Send + 'static, name: String, prefix: &'static str,
) -> JoinHandle<()> {
    spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            eprintln!("[{name}:{prefix}] {line}");
        }
    })
}

/// Start the example server and wait for it to be ready
///
/// This function:
/// 1. Locates the WASM and host binaries
/// 2. Spawns the server process with stdout/stderr capture
/// 3. Waits for the server to accept HTTP connections
/// 4. Verifies HTTP readiness with a HEAD request
/// 5. Returns a handle that will cleanup the server on drop
///
/// The server is given up to SERVER_STARTUP_TIMEOUT to become ready.
fn start_server(example: &str, port: u16) -> Result<ServerHandle, String> {
    let wasm_name = example.replace('-', "_");
    let wasm_file = repo_root()
        .join("target/wasm32-wasip2/debug/examples")
        .join(format!("{wasm_name}_wasm.wasm"));

    if !wasm_file.exists() {
        return Err(format!(
            "WASM file not found: {} (did you build the example?)",
            wasm_file.display()
        ));
    }

    let host_binary = repo_root().join("target/debug/examples").join(example);

    if !host_binary.exists() {
        return Err(format!(
            "Host binary not found: {} (did you build the example?)",
            host_binary.display()
        ));
    }

    eprintln!("Starting server {example} on port {port}...");

    let wasm_file_path = wasm_file
        .to_str()
        .ok_or_else(|| format!("WASM file path contains invalid UTF-8: {}", wasm_file.display()))?;

    let mut child = Command::new(&host_binary)
        .current_dir(repo_root())
        .args(["run", wasm_file_path])
        .env("HTTP_ADDR", format!("0.0.0.0:{port}"))
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start server {example}: {e}"))?;

    // Wait for server to be ready
    let start = Instant::now();

    // Spawn threads to capture stdout/stderr
    let stderr_thread =
        child.stderr.take().map(|stderr| logger(stderr, example.to_string(), "stderr"));

    let stdout_thread =
        child.stdout.take().map(|stdout| logger(stdout, example.to_string(), "stdout"));

    while start.elapsed() < SERVER_STARTUP_TIMEOUT {
        // Check if process is still alive
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("Server {} exited early with status: {status}", example));
            }
            Ok(None) => {} // Still running
            Err(e) => {
                return Err(format!("Error checking server {} status: {e}", example));
            }
        }

        // Try to connect to the port and verify HTTP readiness
        let addr = format!("127.0.0.1:{port}")
            .parse()
            .map_err(|e| format!("Invalid address 127.0.0.1:{port}: {e}"))?;

        if let Ok(mut stream) = TcpStream::connect_timeout(&addr, HTTP_CONNECT_TIMEOUT) {
            // Verify HTTP readiness by making a simple HEAD request
            // This ensures the server is not just listening but actually processing HTTP
            if stream.set_read_timeout(Some(HTTP_CONNECT_TIMEOUT)).is_err()
                || stream.set_write_timeout(Some(HTTP_CONNECT_TIMEOUT)).is_err()
            {
                // Continue to next iteration if we can't set timeouts
                sleep(SERVER_READINESS_CHECK_INTERVAL);
                continue;
            }

            // Send a minimal HEAD request to verify HTTP readiness
            if write!(
                stream,
                "HEAD / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
            )
            .is_ok()
                && stream.flush().is_ok()
            {
                let mut buffer = [0u8; 1024];
                match stream.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        eprintln!("Server {} is ready on port {port}", example);
                        return Ok(ServerHandle::new(child, example, stdout_thread, stderr_thread));
                    }
                    _ => {
                        // Response not ready, continue waiting
                    }
                }
            }
        }

        sleep(SERVER_READINESS_CHECK_INTERVAL);
    }

    // Timeout - kill the process and join threads
    let _ = child.kill();
    let _ = child.wait();
    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }
    Err(format!("Server failed to start within {} seconds", SERVER_STARTUP_TIMEOUT.as_secs()))
}

/// Make an HTTP request to test the server
///
/// This creates a raw TCP connection and sends an HTTP/1.1 request.
/// The response is read and parsed to check for a 2xx status code.
///
/// Using raw TCP instead of a HTTP library gives us more control and
/// reduces test dependencies.
fn test_endpoint(
    port: u16, method: HttpMethod, path: &str, body: Option<&str>,
) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}{path}");
    eprintln!("Testing endpoint: {method:?} {url}");

    // Connect to server
    let addr = format!("127.0.0.1:{port}").parse().map_err(|e| format!("Invalid address: {e}"))?;

    let mut client = TcpStream::connect_timeout(&addr, HTTP_REQUEST_TIMEOUT)
        .map_err(|e| format!("Failed to connect to {url}: {e}"))?;

    client
        .set_read_timeout(Some(HTTP_REQUEST_TIMEOUT))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;
    client
        .set_write_timeout(Some(HTTP_REQUEST_TIMEOUT))
        .map_err(|e| format!("Failed to set write timeout: {e}"))?;

    // Build HTTP request - pre-allocate buffer for better performance
    let mut request = String::with_capacity(256 + body.map_or(0, |b| b.len()));
    request.push_str(method.as_str());
    request.push(' ');
    request.push_str(path);
    request.push_str(" HTTP/1.1\r\nHost: 127.0.0.1:");
    request.push_str(&port.to_string());
    request.push_str("\r\n");

    if let Some(body_bytes) = body {
        request.push_str("Content-Type: application/json\r\nContent-Length: ");
        request.push_str(&body_bytes.len().to_string());
        request.push_str("\r\n");
    }

    request.push_str("Connection: close\r\n\r\n");

    if let Some(body_bytes) = body {
        request.push_str(body_bytes);
    }

    // Send request
    client
        .write_all(request.as_bytes())
        .and_then(|_| client.flush())
        .map_err(|e| format!("Failed to send request to {url}: {e}"))?;

    // Read response with pre-allocated buffer
    let mut response = Vec::with_capacity(4096);
    let mut buffer = [0u8; 4096];

    loop {
        match client.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => response.extend_from_slice(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(format!("Failed to read response from {url}: {e}")),
        }

        // Limit response size to prevent memory issues
        if response.len() > 1_048_576 {
            // 1MB max
            break;
        }
    }

    if response.is_empty() {
        return Err(format!("Empty response from {url}"));
    }

    // Parse response efficiently
    let response_str = String::from_utf8_lossy(&response);
    let status_line =
        response_str.lines().next().ok_or_else(|| format!("Empty response from {url}"))?;

    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("No status code in response from {url}"))?
        .parse()
        .map_err(|e| format!("Invalid status code in response from {url}: {e}"))?;

    eprintln!("Response status: {status_code}");

    if (200..300).contains(&status_code) {
        Ok(())
    } else {
        // Truncate response body if too long for error message
        let error_body = if response_str.len() > 1000 {
            format!("{}... (truncated)", &response_str[..1000])
        } else {
            response_str.into_owned()
        };
        Err(format!("Request to {url} failed with status {status_code}:\n{error_body}"))
    }
}

/// Run a single example test
fn run_example_test(config: &ExampleConfig) -> Result<(), String> {
    let start_time = Instant::now();

    eprintln!("\n{}", "=".repeat(50));
    eprintln!("Testing: {}", config.name);
    eprintln!("{}", "=".repeat(50));

    // Check required environment variables
    if let Err(missing) = check_required_env(config.required_env) {
        return Err(format!("Missing required environment variables: {}", missing.join(", ")));
    }

    // Check Docker requirements
    if config.needs_docker
        && let Some(compose_file) = config.docker_compose
        && !docker_service_running(compose_file)
    {
        return Err(format!(
            "Docker services not running. Start with: docker compose -f {compose_file} up -d"
        ));
    }

    // Ensure this example is built
    let build_start = Instant::now();
    ensure_example_built(config.name)?;
    eprintln!("Build time: {:.2}s", build_start.elapsed().as_secs_f64());

    // If build-only, we're done
    if config.build_only {
        eprintln!("✓ Build-only test passed for {}", config.name);
        return Ok(());
    }

    // Get a free port
    let port = get_free_port();
    eprintln!("Using port: {port}");

    // Start server
    let server_start = Instant::now();
    let _server = start_server(config.name, port)?;
    eprintln!("Server startup time: {:.2}s", server_start.elapsed().as_secs_f64());

    // Small delay to ensure server is fully ready after TCP connection
    sleep(SERVER_READINESS_DELAY);

    // Test the endpoint
    let request_start = Instant::now();
    test_endpoint(port, config.method, config.path, config.body)?;
    eprintln!("Request time: {:.2}s", request_start.elapsed().as_secs_f64());

    eprintln!("✓ Test passed: {} (total: {:.2}s)", config.name, start_time.elapsed().as_secs_f64());
    Ok(())
}

// ============================================================================
// Individual test functions for each example
// ============================================================================

/// Macro to generate a test for a standalone example
///
/// This generates a test function that:
/// - Is marked with #[ignore] since integration tests are slow
/// - Automatically skips if environment variables are missing
/// - Automatically skips if Docker services aren't running
/// - Panics on actual test failures
macro_rules! example_test {
    ($test_name:ident, $example:expr) => {
        #[test]
        #[ignore = "Too slow"]
        fn $test_name() {
            let example = $example;
            let config = get_example_configs().get(example).unwrap_or_else(|| {
                panic!(
                    "Invalid example name: {example}. Available examples: {:?}",
                    get_example_configs().keys()
                );
            });

            match run_example_test(config) {
                Ok(()) => {}
                Err(e) if e.contains("Missing required environment") => {
                    eprintln!("Skipping {example}: {e}");
                }
                Err(e) if e.contains("Docker services not running") => {
                    eprintln!("Skipping {example}: {e}");
                }
                Err(e) => panic!("Test failed for {example}: {e}"),
            }
        }
    };
}

/// Macro to generate a test for a Docker-dependent example
///
/// This generates a test function that:
/// - Is marked with #[ignore] with instructions to start Docker services
/// - Fails immediately if Docker services aren't available
/// - Panics on any test failure
macro_rules! docker_example_test {
    ($test_name:ident, $example:expr, $compose_file:expr) => {
        #[test]
        #[ignore = concat!("Requires Docker: docker compose -f ", $compose_file, " up -d")]
        fn $test_name() {
            let example = $example;
            let config = get_example_configs().get(example).unwrap_or_else(|| {
                panic!(
                    "Invalid example name: {example}. Available examples: {:?}",
                    get_example_configs().keys()
                );
            });

            match run_example_test(config) {
                Ok(()) => {}
                Err(e) => panic!("Test failed for {example}: {e}"),
            }
        }
    };
}

// Standalone examples (always run)
example_test!(test_blobstore, "blobstore");
example_test!(test_http, "http");
example_test!(test_keyvalue, "keyvalue");
example_test!(test_otel, "otel");
example_test!(test_vault, "vault");
example_test!(test_http_proxy, "http-proxy");
example_test!(test_websockets, "websockets");

// Build-only examples
example_test!(test_messaging, "messaging");
example_test!(test_sql, "sql");

// Examples requiring environment variables (skip if not set)
example_test!(test_identity, "identity");

// Docker-dependent examples (skip if Docker not running)
docker_example_test!(test_blobstore_mongodb, "blobstore-mongodb", "docker/mongodb.yaml");
docker_example_test!(test_blobstore_nats, "blobstore-nats", "docker/nats.yaml");
docker_example_test!(test_keyvalue_nats, "keyvalue-nats", "docker/nats.yaml");
docker_example_test!(test_keyvalue_redis, "keyvalue-redis", "docker/redis.yaml");
docker_example_test!(test_messaging_kafka, "messaging-kafka", "docker/kafka.yaml");
docker_example_test!(test_messaging_nats, "messaging-nats", "docker/nats.yaml");
docker_example_test!(test_sql_postgres, "sql-postgres", "docker/postgres.yaml");
docker_example_test!(test_vault_azure, "vault-azure", "docker/azurekv.yaml");
