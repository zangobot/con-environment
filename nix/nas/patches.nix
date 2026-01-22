{ pkgs, lib, inputs, nfsServer, nfsPath }:
let
  # Import the manifest defined in Step 1
  patches = import ../patches/manifest.nix { 
    inherit pkgs lib inputs; 
    nfsServer
    nfsPath
  };
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

  cp -f -r "${config}" "$TARGET_DIR/config"

  echo "✅ Done."
''