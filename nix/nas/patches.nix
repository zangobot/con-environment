{ pkgs, lib, inputs, patchDir, ... }:
let
  # 1. Configuration Constants
  patchesSrc = ../patches;
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };

  staticPatchFiles = lib.filterAttrs 
    (name: type: type == "regular" && lib.hasSuffix ".yaml" name) 
    (builtins.readDir patchesSrc);

  copyStaticCmds = lib.concatStringsSep "\n" (lib.mapAttrsToList (name: _: ''
    echo "📄 Copying static patch: ${name}"
    cp "${patchesSrc}/${name}" "${patchDir}/${name}"
  '') staticPatchFiles);

  ciliumGenerator = import ../patches/cilium.nix {
    inherit pkgs kubelib;
    output = "${patchDir}/cilium.yaml";
  };
  ghcr-auth = import ../patches/ghcr.nix {
    inherit pkgs kubelib;
    output = "${patchDir}/ghcr.yaml";
  };

  
  generatePatches = pkgs.writeShellScriptBin "generate-patches" ''
    set -euo pipefail
    echo "🚀 Starting Patch Generation..."
    echo "   Target: ${patchDir}"
    
    mkdir -p "${patchDir}"

    ${lib.getExe ciliumGenerator}
    ${lib.getExe ghcr-auth}

    ${copyStaticCmds}

    echo "✅ All patches generated successfully."
  '';

in
{
  # Expose the script to the system
  environment.systemPackages = [ generatePatches ];
}