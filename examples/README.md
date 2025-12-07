# Examples

The `examples` directory contains several example projects demonstrating each WASI component
in combination with implemented resources. For example `wasi-keyvalue` combined with either
a Redis or NATS JetStream backend.

Each example now includes a dedicated host runtime binary so you can run it with
`cargo run --bin <example-runtime> ...` without enabling unrelated features. See
the individual README files for the exact command lines.

