# Docker Compose Resources

## Publishing to GitHub Container Registry


### Login

```bash
 docker login ghcr.io -u $GITHUB_USERNAME --password $GITHUB_TOKEN
```

### Publish

```bash
docker compose -f docker/otelcol.yaml publish ghcr.io/credibil/compose-otelcol:latest
docker compose -f docker/kafka.yaml publish --with-env ghcr.io/credibil/compose-kafka:latest
docker compose -f docker/mongodb.yaml publish --with-env ghcr.io/credibil/compose-mongodb:latest
docker compose -f docker/nats.yaml publish --with-env ghcr.io/credibil/compose-nats:latest
docker compose -f docker/postgres.yaml publish --with-env ghcr.io/credibil/compose-postgres:latest
docker compose -f docker/redis.yaml publish --with-env ghcr.io/credibil/compose-redis:latest
```