{ pkgs, lib, inputs, clusterName, talosVersion, vIp, ... }:

let
  clusterEndpoint = "https://${vIp}:6443";
  patchesSet = import ../patches/manifest.nix { inherit pkgs lib inputs; };
  patchFlags = lib.concatMapStringsSep " " (p: "--config-patch @${p}") [
    patchesSet.cilium
    patchesSet.ghcr
    patchesSet.install
  ];
  controlPatchFlags = lib.concatMapStringsSep " " (p: "--config-patch-control-plane @${p}") [
    patchesSet.control-schedule
  ];
in
pkgs.runCommand "talos-config" {
  nativeBuildInputs = [ pkgs.talosctl ];
} ''
  # Create the output directory
  mkdir -p $out

  echo "🚀 Generating Talos Configuration for ${clusterName}..."
  echo "Applying patches:"
  echo "${patchFlags}"

  # Generate the configuration
  # We use the flags to apply patches immediately during generation
  talosctl gen config \
    "${clusterName}" \
    "${clusterEndpoint}" \
    --install-disk "" \
    --output "$out" \
    --talos-version "${talosVersion}" \
    ${patchFlags} \
    ${controlPatchFlags}

  echo "✅ Configuration generated in $out"
''