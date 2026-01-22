{ pkgs, lib, inputs, nfsServer, nfsPath, ... }:
let
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };
  nfsFile = import ./nfs.nix {
    inherit pkgs kubelib;
    server = nfsServer;
    path = nfsPath;
  };
  nvidiaFile = import ./nvidia.nix {
    inherit pkgs kubelib;
  };

in {
    all = [
      (ciliumFile)
      (ghcrAuthFile)
      (nfsFile)
      (nvidiaFile)
      ./install.yaml
    ];
    control = [./control/schedule.yaml];
    worker = [];
  }