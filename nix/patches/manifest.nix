{ pkgs, lib, inputs, ... }:
let
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };
  nfsFile = import ./nfs.nix {
    inherit pkgs;
  };
  nvidiaFile = import ./nvidia.nix {
    inherit pkgs;
  };

in {
    all = [
      (ciliumFile)
      (ghcrAuthFile)
      ./install.yaml
    ];
    control = [./control/schedule.yaml];
    worker = [];
  }