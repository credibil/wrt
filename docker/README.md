# Docker Compose Resources

## Publishing to GitHub Container Registry


### Login

```bash
 docker login ghcr.io -u $GITHUB_USERNAME --password $GITHUB_TOKEN
```

### Publish

```bash
# OpenTelemetry
docker compose -f docker/opentelemetry.yaml publish ghcr.io/credibil/compose-opentelemetry:latest

# Resources
docker compose -f docker/resources.yaml publish --with-env ghcr.io/credibil/compose-resources:latest
```