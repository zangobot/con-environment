{ pkgs, lib, inputs }:
let
  # Import the manifest defined in Step 1
  patches = import ../patches/manifest.nix { inherit pkgs lib inputs; };
  config = import ./talos-config.nix { 
    inherit pkgs lib inputs; 
    clusterName = "aivProd";
    talosVersion = "v1.12.1";
    vIp = "10.211.0.20";
  };

in
pkgs.writeShellScriptBin "generate-patches" ''
  set -euo pipefail
  TARGET_DIR="''${1}" # Default to ./patches, allow override
  
  echo "🚀 Generating patches to: $TARGET_DIR"
  mkdir -p "$TARGET_DIR"

  cp -f "${patches.cilium}" "$TARGET_DIR/cilium.yaml"
  cp -f "${patches.control-schedule}" "$TARGET_DIR/control-schedule.yaml"
  cp -f "${patches.ghcr}" "$TARGET_DIR/ghcr.yaml"
  cp -f "${patches.install}" "$TARGET_DIR/install.yaml"
  cp -f -r "${config}" "$TARGET_DIR/config"

  echo "✅ Done."
''