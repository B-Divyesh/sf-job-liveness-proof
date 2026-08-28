#!/usr/bin/env bash
set -euo pipefail

expected_sha="${1:-$(git rev-parse HEAD)}"
resource_group="${RUN_PROOF_RESOURCE_GROUP:-sociobot}"
app_name="${RUN_PROOF_CONTAINER_APP:-sf-job-liveness-proof}"
url="${RUN_PROOF_LIVE_URL:-https://job-liveness-proof.sociobot.in}"

state="$(az containerapp show --resource-group "$resource_group" --name "$app_name" --output json)"
jq -e '
  .properties.template.scale.minReplicas == 1 and
  .properties.template.scale.maxReplicas == 1 and
  (.properties.template.volumes | any(
    .name == "run-proof-data" and
    .storageName == "data-job-liveness-proof" and
    .storageType == "AzureFile"
  )) and
  (.properties.template.containers[0].volumeMounts | any(
    .volumeName == "run-proof-data" and .mountPath == "/data"
  ))
' <<<"$state" >/dev/null

health="$(curl --fail --silent --show-error "${url}/health")"
jq -e --arg sha "$expected_sha" '.status == "ok" and .build_sha == $sha' <<<"$health" >/dev/null

echo "deployment verified: sha=${expected_sha} replicas=1 durable_mount=/data"
