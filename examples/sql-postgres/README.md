# SQL with Postgres Example

This demonstrates making a call to HTTP end-points that in turn execute a SQL query on a Postgres server.

The extra files in this folder assume the guest will be run on the `rt_standard` runtime so `compose.yaml` and associated configuration files allow that full resource set.

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

In a console, build and run the `postgres` example:

```bash
# build the guest.
cargo build --example postgres --target wasm32-wasip2 --release

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/postgres/compose.yaml up
```

Use Postman or in a separate console, call the guest with a POST request to populate some data in a Postgres table:

```bash
curl -X POST --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/
```

Then execute the following to retrieve the data:

```bash
curl http://localhost:8080/
```
