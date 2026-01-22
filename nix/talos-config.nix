{ pkgs, lib, inputs, clusterName, talosVersion, vIp, nfsServer, mainPath, vllmPath, ... }:

let
  clusterEndpoint = "https://${vIp}:6443";
  patchesSet = import ../patches/manifest.nix { inherit pkgs lib inputs nfsServer mainPath vllmPath; };
  patchFlags = lib.concatMapStringsSep " " (p: "--config-patch @${p}") patchesSet.all;
  controlPatchFlags = lib.concatMapStringsSep " " (p: "--config-patch-control-plane @${p}") patchesSet.control;
  workerPatchFlags = lib.concatMapStringsSep " " (p: "--config-patch-control-plane @${p}") patchesSet.worker;
in
pkgs.writeShellScript "generate-talos-config.sh" ''
  set -e
  SECRETS_FILE=$1
  OUTPUT_DIR=$2

  if [ -z "$SECRETS_FILE" ] || [ -z "$OUTPUT_DIR" ]; then
    echo "Usage: $0 <path-to-secrets> <output-dir>"
    exit 1
  fi

  # Ensure directory exists
  mkdir -p "$OUTPUT_DIR"
  cd "$OUTPUT_DIR"

  echo "🚀 Generating Talos Configuration for ${clusterName}..."
  
  # We reference ${pkgs.talosctl} directly so it's available on the NAS
  ${pkgs.talosctl}/bin/talosctl gen config \
    "${clusterName}" \
    "${clusterEndpoint}" \
    --install-disk "/dev/sda" \
    --talos-version "${talosVersion}" \
    ${patchFlags} \
    ${controlPatchFlags} \
    ${workerPatchFlags} \
    --with-secrets "$SECRETS_FILE" \
    --force

  # Fix permissions so your admin user can read them (optional)
  chmod 644 controlplane.yaml worker.yaml talosconfig
  echo "✅ Configuration generated in $OUTPUT_DIR"
''