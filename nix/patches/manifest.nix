{ pkgs, lib, inputs, ... }:
let

  rawFiles = builtins.readDir ./.;
  yamlFiles = lib.filterAttrs 
    (name: type: type == "regular" && lib.hasSuffix ".yaml" name) 
    rawFiles;
  staticPatchFiles = lib.mapAttrs (name: _: ./. + "/${name}") yamlFiles;

  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };

in staticPatchFiles // {
    "cilium.yaml" = (ciliumFile);
    "ghcr.yaml" = (ghcrAuthFile);
  }