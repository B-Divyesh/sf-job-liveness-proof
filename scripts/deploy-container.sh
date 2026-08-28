#!/usr/bin/env bash
set -euo pipefail

# Factory deployment for the SQLite artifact. Updating only the image is unsafe:
# every rollout must also reassert the durable /data mount and one-replica limit.
sha="${1:-$(git rev-parse HEAD)}"
if [[ ! "$sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "usage: $0 <full-40-character-commit-sha>" >&2
  exit 2
fi

resource_group="${RUN_PROOF_RESOURCE_GROUP:-sociobot}"
app_name="${RUN_PROOF_CONTAINER_APP:-sf-job-liveness-proof}"
registry="${RUN_PROOF_REGISTRY:-sociobotregistry}"
storage_name="${RUN_PROOF_ENV_STORAGE:-data-job-liveness-proof}"
tag="${sha:0:12}"
image="${registry}.azurecr.io/${app_name}:${tag}"

az containerapp env storage show \
  --resource-group "$resource_group" \
  --name factory-env \
  --storage-name "$storage_name" \
  --output none

az acr build \
  --registry "$registry" \
  --image "${app_name}:${tag}" \
  --build-arg "BUILD_SHA=${sha}" \
  .

current="$(az containerapp show --resource-group "$resource_group" --name "$app_name" --output json)"
resource_id="$(jq -r '.id' <<<"$current")"
patch="$(jq \
  --arg image "$image" \
  --arg storage "$storage_name" \
  '{properties:{template:{
    containers:[(.properties.template.containers[0]
      | .image=$image
      | .env=[{"name":"PORT","value":"8080"}]
      | .volumeMounts=[{"mountPath":"/data","volumeName":"run-proof-data"}])],
    scale:{"minReplicas":1,"maxReplicas":1,"rules":null},
    volumes:[{"name":"run-proof-data","storageName":$storage,"storageType":"AzureFile"}]
  }}}' <<<"$current")"

az rest \
  --method patch \
  --url "${resource_id}?api-version=2024-03-01" \
  --headers 'Content-Type=application/json' \
  --body "$patch" \
  --output none

for _ in $(seq 1 60); do
  if ./scripts/verify-deployment.sh "$sha" >/dev/null 2>&1; then
    ./scripts/verify-deployment.sh "$sha"
    exit 0
  fi
  sleep 5
done

echo "deployment did not reach the required identity and topology" >&2
./scripts/verify-deployment.sh "$sha"
exit 1
