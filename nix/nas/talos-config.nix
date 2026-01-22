{ pkgs, lib, inputs, clusterName, talosVersion, vIp, nfsServer, mainPath, vllmPath, ... }:

let
  clusterEndpoint = "https://${vIp}:6443";
  patchesSet = import ../patches/manifest.nix { inherit pkgs lib inputs nfsServer mainPath vllmPath; };
  patchFlags = lib.concatMapStringsSep " " (p: "--config-patch @${p}") patchesSet.all;
  controlPatchFlags = lib.concatMapStringsSep " " (p: "--config-patch-control-plane @${p}") patchesSet.control;
  workerPatchFlags = lib.concatMapStringsSep " " (p: "--config-patch-control-plane @${p}") patchesSet.worker;
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
    ${controlPatchFlags} \
    ${workerPatchFlags}

  echo "✅ Configuration generated in $out"
''