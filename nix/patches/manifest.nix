{ pkgs, lib, inputs, ... }:
let
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };

in {
    install = ./install.yaml;
    control-schedule = ./control/schedule.yaml;
    cilium = (ciliumFile);
    ghcr = (ghcrAuthFile);
  }