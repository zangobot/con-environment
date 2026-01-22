{ pkgs ? import <nixpkgs> {} }:
{
  # --- Required Arguments ---
  version, # e.g., "v1.9.0"
  
  # --- Hash (Update this after running the helper script) ---
  sha256 ? "", 

  # --- Schematic ID (Update this after running the helper script) ---
  # If null, we attempt to calculate it (unreliable) or use vanilla.
  schematic ? null,

  # --- Configuration ---
  platform ? "metal", 
  arch ? "amd64",
  secureboot ? false,
  
  # --- Extensions ---
  # Default to the requested Intel + Nvidia stack
  systemExtensions ? [
    "siderolabs/amd-ucode"
    "siderolabs/intel-ucode" 
    "siderolabs/nvidia-open-gpu-kernel-modules"
    "siderolabs/nvidia-container-toolkit" 
  ],
  
  extraKernelArgs ? [],
  meta ? {}
}:

let
  # 1. Define the Schematic
  schematicConfig = {
    customization = {
      systemExtensions = {
        officialExtensions = systemExtensions;
      };
      extraKernelArgs = extraKernelArgs;
      meta = meta;
    };
  };

  schematicJson = builtins.toJSON schematicConfig;

  # 2. Determine Schematic ID
  # Ideally, provided manually. If not, we fallback to a local hash (often inaccurate vs Factory).
  defaultSchematicId = "376567988ad370138ad8b2698212367b8edcb69b5fd68c80be1f2ec7d603b4ba";
  computedSchematicId = 
    if schematic != null then schematic
    else if (systemExtensions == [] && extraKernelArgs == [] && meta == {}) then defaultSchematicId
    else builtins.hashString "sha256" schematicJson; # Note: This often mismatches the Factory's ID

  # 3. Construct Filename
  secbootSuffix = if secureboot then "-secureboot" else "";
  fileName = "${platform}-${arch}${secbootSuffix}.iso";
  
  # 4. Construct URL
  factoryUrl = "https://factory.talos.dev/image/${computedSchematicId}/${version}/${fileName}";

in
pkgs.fetchurl {
  name = "talos-${version}-${fileName}";
  url = factoryUrl;
  inherit sha256;

  passthru = {
    inherit schematicConfig computedSchematicId;

    # --- The "One-Stop" Helper Script ---
    # Runs on your host to register the config and get the correct Hashes
    updateScript = pkgs.writeShellScript "fetch-talos-info" ''
      set -e
      export PATH="${pkgs.curl}/bin:${pkgs.jq}/bin:${pkgs.nix}/bin:$PATH"

      echo "-----------------------------------------------------"
      echo "Step 1: Registering Schematic with Talos Factory..."
      echo "-----------------------------------------------------"
      
      # Post the JSON to the factory
      RESPONSE=$(curl -s -X POST --data-binary '${schematicJson}' https://factory.talos.dev/schematics)
      
      # Extract ID (The factory returns {"id": "..."})
      ID=$(echo "$RESPONSE" | jq -r '.id')
      
      if [ "$ID" == "null" ] || [ -z "$ID" ]; then
        echo "Error: Failed to get ID from factory. Response:"
        echo "$RESPONSE"
        exit 1
      fi
      
      echo "Got Schematic ID: $ID"
      
      echo ""
      echo "-----------------------------------------------------"
      echo "Step 2: Prefetching ISO to calculate SHA256..."
      echo "-----------------------------------------------------"
      
      URL="https://factory.talos.dev/image/$ID/${version}/${fileName}"
      echo "URL: $URL"
      
      # Prefetch the file to get the hash
      HASH=$(nix-prefetch-url "$URL")
      
      echo ""
      echo "====================================================="
      echo "  UPDATE YOUR NIX EXPRESSION WITH THESE VALUES:"
      echo "====================================================="
      echo ""
      echo "  schematic = \"$ID\";"
      echo "  sha256    = \"$HASH\";"
      echo ""
      echo "====================================================="
    '';
  };
}