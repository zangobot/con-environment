{ pkgs, lib, inputs }:
let
  # Import the manifest defined in Step 1
  patches = import ./manifest.nix { inherit pkgs lib inputs; };

  # Generate 'cp' commands for every file in the set
  installCommands = lib.concatStringsSep "\n" (lib.mapAttrsToList (name: src: ''
    echo "📄 Installing ${name}..."
    cp -f "${src}" "$TARGET_DIR/${name}"
    chmod 600 "$TARGET_DIR/${name}"
  '') patches);
in
pkgs.writeShellScriptBin "generate-patches" ''
  set -euo pipefail
  TARGET_DIR="''${1}" # Default to ./patches, allow override
  
  echo "🚀 Generating patches to: $TARGET_DIR"
  mkdir -p "$TARGET_DIR"

  ${installCommands}

  echo "✅ Done."
''