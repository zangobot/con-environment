{ pkgs, lib, inputs, ... }:
let
  install = ./install.yaml;
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };

in {
    "install.yaml" = install;
    "cilium.yaml" = (ciliumFile);
    "ghcr.yaml" = (ghcrAuthFile);
  }