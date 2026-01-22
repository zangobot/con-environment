{ pkgs ? import <nixpkgs> {} }:
{
  version,   # e.g. "v1.9.0"
  sha256,    # The hash of the final image
  diskImage ? "iso",
  platform ? "metal",
  arch ? "amd64",
  secureboot ? false, 
  systemExtensions ? [],
  extraKernelArgs ? [],
  meta ? []
}:

let
  # 1. Define the Configuration
  schematicConfig = {
    customization = {
      systemExtensions = {
        officialExtensions = systemExtensions;
      };
      extraKernelArgs = extraKernelArgs;
      meta = meta;
    } // pkgs.lib.optionalAttrs secureboot {
      secureboot = { includeWellKnownCertificates = true; };
    };
  };

  schematicJson = builtins.toJSON schematicConfig;
  secbootSuffix = if secureboot then "-secureboot" else "";

  imageConfig = if diskImage == "iso" then {
    ext = ".iso";
    domain = "factory.talos.dev";
    pathPrefix = "image";
  } else if diskImage == "raw" then {
    ext = ".raw.zst";
    domain = "factory.talos.dev";
    pathPrefix = "image";
  } else if diskImage == "qcow2" then {
    ext = ".qcow2";
    domain = "factory.talos.dev";
    pathPrefix = "image";
  } else if diskImage == "pxe" then {
    ext = "";
    domain = "pxe.factory.talos.dev";
    pathPrefix = "pxe";
  } else abort "Unknown diskImage type: ${diskImage}. Supported: iso, raw, qcow2, pxe";

  baseName = "${platform}-${arch}${secbootSuffix}";
  fileName = "${baseName}${imageConfig.ext}";

in
pkgs.stdenvNoCC.mkDerivation {
  name = "talos-${version}-${fileName}";
  outputHashAlgo = "sha256";
  outputHash     = sha256;
  
  # "flat" means the output is a single file, not a directory
  outputHashMode = "flat"; 
  nativeBuildInputs = [ pkgs.curl pkgs.jq ];
  buildCommand = ''
    export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
    export PATH="${pkgs.curl}/bin:${pkgs.jq}/bin:$PATH"
    
    echo "--> Registering Talos Schematic..."
    echo "    Config: ${schematicJson}"

    # --- STEP 1: POST to get the ID ---
    RESPONSE=$(curl -s -X POST \
      -H "Content-Type: application/json" \
      --data-binary '${schematicJson}' \
      https://factory.talos.dev/schematics)
    echo "$RESPONSE"  
    ID=$(echo "$RESPONSE" | jq -r '.id')

    if [ -z "$ID" ] || [ "$ID" == "null" ]; then
      echo "Error: Failed to retrieve Schematic ID. Factory response:"
      echo "$RESPONSE"
      exit 1
    fi

    echo "--> Success! Got Schematic ID: $ID"

    # --- STEP 2: Download the ISO using that ID ---
    URL="https://${imageConfig.domain}/${imageConfig.pathPrefix}/$ID/${version}/${fileName}"
    echo "--> Downloading ISO from: $URL"
    curl -L --fail --show-error --progress-bar -o $out "$URL"
  '';
}