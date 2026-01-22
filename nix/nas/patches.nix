{ pkgs, lib, inputs, nfsServer, mainPath, vllmPath }:
let
  # Import the manifest defined in Step 1
  patches = import ../patches/manifest.nix { 
    inherit pkgs lib inputs nfsServer mainPath vllmPath;
  };
  config = import ./talos-config.nix { 
    inherit pkgs lib inputs nfsServer mainPath vllmPath;
    clusterName = "aivProd";
    talosVersion = "v1.12.1";
    vIp = "10.211.0.20";
  };
  controlCopyScript = pkgs.lib.strings.concatStringsSep "\n" (
    pkgs.lib.lists.imap1 (i: src: ''
      cp -f -r "${src}" "$TARGET_DIR/control/${toString i}.yaml"
    '') patches.control
  );

  allCopyScript = pkgs.lib.strings.concatStringsSep "\n" (
    pkgs.lib.lists.imap1 (i: src: ''
      cp -f -r "${src}" "$TARGET_DIR/all/${toString i}.yaml"
    '') patches.all
  );

  workerCopyScript = pkgs.lib.strings.concatStringsSep "\n" (
    pkgs.lib.lists.imap1 (i: src: ''
      cp -f -r "${src}" "$TARGET_DIR/worker/${toString i}.yaml"
    '') patches.worker
  );
in
pkgs.writeShellScriptBin "generate-patches" ''
  set -euo pipefail
  TARGET_DIR="''${1}" # Default to ./patches, allow override
  
  echo "🚀 Generating patches to: $TARGET_DIR"
  mkdir -p "$TARGET_DIR"
  mkdir -p "$TARGET_DIR/control"
  mkdir -p "$TARGET_DIR/all"
  mkdir -p "$TARGET_DIR/worker"

  ${controlCopyScript}
  ${allCopyScript}
  ${workerCopyScript}
  cp -f -r "${config}" "$TARGET_DIR/config"

  echo "✅ Done."
''