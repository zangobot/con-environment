{ pkgs, lib, inputs, ... }:
let
  # 1. Configuration Constants
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };

  staticPatchFiles = lib.filterAttrs 
    (name: type: type == "regular" && lib.hasSuffix ".yaml" name) 
    (builtins.readDir ./.);

  ciliumFile = import .//cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import .//ghcr.nix {
    inherit pkgs;
  };
in staticPatchFiles // {
    "cilium.yaml" = ciliumFile;
    "ghcr.yaml" = ghcrAuthFile;
  }