//! Integration tests for WASI examples
//!
//! These tests build and run the example servers, making HTTP requests to verify
//! they work correctly.
//!
//! Run with: `cargo test --test examples`
//! Run specific test: `cargo test --test examples test_http`
//! Run with output: `cargo test --test examples -- --nocapture`

use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Configuration for testing an example
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

#[derive(Debug, Clone, Copy)]
enum HttpMethod {
    Get,
    Post,
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

/// All example configurations
fn get_example_configs() -> HashMap<&'static str, ExampleConfig> {
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
}

/// Get the repository root directory
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Find a free port to use for testing
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
        vars.iter().filter(|var| env::var(var).is_err()).map(|s| (*s).to_string()).collect();

    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

/// Check if Docker compose services are running
fn docker_service_running(compose_file: &str) -> bool {
    let compose_path = repo_root().join(compose_file);
    if !compose_path.exists() {
        return false;
    }

    Command::new("docker")
        .args(["compose", "-f", compose_path.to_str().unwrap(), "ps", "--status", "running"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check if there are any running containers (more than just the header)
            stdout.lines().count() > 1
        })
        .unwrap_or(false)
}

/// Track which examples have been built
static BUILT_EXAMPLES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

/// Build a specific example (WASM guest and host)
fn ensure_example_built(example: &str) -> Result<(), String> {
    // Check if already built
    {
        let built = BUILT_EXAMPLES.lock().unwrap();
        if built.contains(example) {
            return Ok(());
        }
    }

    eprintln!("Building example: {example}...");

    // Build WASM guest
    let wasm_example = format!("{example}-wasm");
    let wasm_result = Command::new("cargo")
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
        .status();

    match wasm_result {
        Ok(status) if status.success() => {}
        Ok(status) => {
            return Err(format!("WASM build failed for {example} with status: {status}"));
        }
        Err(e) => {
            return Err(format!("Failed to run WASM build for {example}: {e}"));
        }
    }

    // Build host
    let host_result = Command::new("cargo")
        .current_dir(repo_root())
        .args(["build", "--example", example, "--message-format=short"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match host_result {
        Ok(status) if status.success() => {
            // Mark as built
            let mut built = BUILT_EXAMPLES.lock().unwrap();
            built.insert(example.to_string());
            Ok(())
        }
        Ok(status) => Err(format!("Host build failed for {example} with status: {status}")),
        Err(e) => Err(format!("Failed to run host build for {example}: {e}")),
    }
}

/// Server handle that kills the process on drop
struct ServerHandle {
    child: Child,
    name: String,
}

impl ServerHandle {
    fn new(child: Child, name: &str) -> Self {
        Self {
            child,
            name: name.to_string(),
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        eprintln!("Stopping server: {}", self.name);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start the example server and wait for it to be ready
fn start_server(example: &str, port: u16) -> Result<ServerHandle, String> {
    let wasm_name = example.replace('-', "_");
    let wasm_file = repo_root()
        .join("target/wasm32-wasip2/debug/examples")
        .join(format!("{wasm_name}_wasm.wasm"));

    if !wasm_file.exists() {
        return Err(format!("WASM file not found: {}", wasm_file.display()));
    }

    let host_binary = repo_root().join("target/debug/examples").join(example);

    if !host_binary.exists() {
        return Err(format!("Host binary not found: {}", host_binary.display()));
    }

    eprintln!("Starting server {} on port {port}...", example);

    let mut child = Command::new(&host_binary)
        .current_dir(repo_root())
        .args(["run", wasm_file.to_str().unwrap()])
        .env("HTTP_ADDR", format!("0.0.0.0:{port}"))
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start server: {e}"))?;

    // Wait for server to be ready
    let timeout = Duration::from_secs(60);
    let start = Instant::now();
    let check_interval = Duration::from_millis(500);

    // Try to read stderr for startup messages
    let stderr = child.stderr.take();
    let stdout = child.stdout.take();

    // Spawn thread to consume stdout/stderr
    if let Some(stderr) = stderr {
        let example_name = example.to_string();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[{example_name}] {line}");
            }
        });
    }
    if let Some(stdout) = stdout {
        let example_name = example.to_string();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[{example_name}] {line}");
            }
        });
    }

    while start.elapsed() < timeout {
        // Check if process is still alive
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("Server exited early with status: {status}"));
            }
            Ok(None) => {} // Still running
            Err(e) => {
                return Err(format!("Error checking server status: {e}"));
            }
        }

        // Try to connect to the port
        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_millis(100),
        )
        .is_ok()
        {
            eprintln!("Server {} is ready on port {port}", example);
            return Ok(ServerHandle::new(child, example));
        }

        std::thread::sleep(check_interval);
    }

    // Timeout - kill the process
    let _ = child.kill();
    Err(format!("Server failed to start within {} seconds", timeout.as_secs()))
}

/// Make an HTTP request to test the server
fn test_endpoint(
    port: u16, method: HttpMethod, path: &str, body: Option<&str>,
) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}{path}");
    let timeout = Duration::from_secs(10);

    eprintln!("Testing endpoint: {method:?} {url}");

    // Use a simple blocking HTTP client
    let client = std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        timeout,
    )
    .map_err(|e| format!("Failed to connect: {e}"))?;

    client
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;
    client
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set write timeout: {e}"))?;

    use std::io::{Read, Write};
    let mut client = client;

    // Build HTTP request
    let method_str = match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
    };

    let body_bytes = body.unwrap_or("");
    let content_length = body_bytes.len();

    let request = if body.is_some() {
        format!(
            "{method_str} {path} HTTP/1.1\r\n\
             Host: 127.0.0.1:{port}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {content_length}\r\n\
             Connection: close\r\n\
             \r\n\
             {body_bytes}"
        )
    } else {
        format!(
            "{method_str} {path} HTTP/1.1\r\n\
             Host: 127.0.0.1:{port}\r\n\
             Connection: close\r\n\
             \r\n"
        )
    };

    client.write_all(request.as_bytes()).map_err(|e| format!("Failed to send request: {e}"))?;

    // Read response
    let mut response = Vec::new();
    client.read_to_end(&mut response).map_err(|e| format!("Failed to read response: {e}"))?;

    let response_str = String::from_utf8_lossy(&response);

    // Parse status code from first line
    let status_line = response_str.lines().next().ok_or("Empty response")?;

    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .ok_or("No status code in response")?
        .parse()
        .map_err(|e| format!("Invalid status code: {e}"))?;

    eprintln!("Response status: {status_code}");

    if (200..300).contains(&status_code) {
        Ok(())
    } else {
        Err(format!("Request failed with status {status_code}:\n{response_str}"))
    }
}

/// Run a single example test
fn run_example_test(config: &ExampleConfig) -> Result<(), String> {
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
    ensure_example_built(config.name)?;

    // If build-only, we're done
    if config.build_only {
        eprintln!("Build-only test passed for {}", config.name);
        return Ok(());
    }

    // Get a free port
    let port = get_free_port();

    // Start server
    let _server = start_server(config.name, port)?;

    // Small delay to ensure server is fully ready
    std::thread::sleep(Duration::from_millis(500));

    // Test the endpoint
    test_endpoint(port, config.method, config.path, config.body)?;

    eprintln!("✓ Test passed: {}", config.name);
    Ok(())
}

// ============================================================================
// Individual test functions for each example
// ============================================================================

macro_rules! example_test {
    ($test_name:ident, $example:expr) => {
        #[test]
        #[ignore = "Too slow"]
        fn $test_name() {
            let configs = get_example_configs();
            let example = $example;
            let config =
                configs.get(example).expect(&format!("Should be valid example: {example}"));

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
#[test]
#[ignore = "Requires Docker: docker compose -f docker/mongodb.yaml up -d"]
fn test_blobstore_mongodb() {
    let configs = get_example_configs();
    let config = configs.get("blobstore-mongodb").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/nats.yaml up -d"]
fn test_blobstore_nats() {
    let configs = get_example_configs();
    let config = configs.get("blobstore-nats").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/nats.yaml up -d"]
fn test_keyvalue_nats() {
    let configs = get_example_configs();
    let config = configs.get("keyvalue-nats").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/redis.yaml up -d"]
fn test_keyvalue_redis() {
    let configs = get_example_configs();
    let config = configs.get("keyvalue-redis").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/kafka.yaml up -d"]
fn test_messaging_kafka() {
    let configs = get_example_configs();
    let config = configs.get("messaging-kafka").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/nats.yaml up -d"]
fn test_messaging_nats() {
    let configs = get_example_configs();
    let config = configs.get("messaging-nats").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/postgres.yaml up -d"]
fn test_sql_postgres() {
    let configs = get_example_configs();
    let config = configs.get("sql-postgres").unwrap();
    run_example_test(config).expect("Test failed");
}

#[test]
#[ignore = "Requires Docker: docker compose -f docker/azurekv.yaml up -d"]
fn test_vault_azure() {
    let configs = get_example_configs();
    let config = configs.get("vault-azure").unwrap();
    run_example_test(config).expect("Test failed");
}
