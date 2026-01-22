{ pkgs, lib, inputs, nfsServer, mainPath, vllmPath, ... }:
let
  kubelib = inputs.nix-kube-generators.lib { inherit pkgs; };
  ciliumFile = import ./cilium.nix {
    inherit pkgs kubelib;
  };
  ghcrAuthFile = import ./ghcr.nix {
    inherit pkgs;
  };
  mainPcvFile = import ./nfs.nix {
    inherit pkgs kubelib;
    server = nfsServer;
    path = mainPath;
  };
  modelPvcFile = import ./nfs.nix {
    inherit pkgs kubelib;
    server = nfsServer;
    path = mainPath;
    name = "vllmPath";
  };
  nvidiaFile = import ./nvidia.nix {
    inherit pkgs kubelib;
  };

in {
    all = [
      (ciliumFile)
      (ghcrAuthFile)
      (mainPcvFile)
      (modelPvcFile)
      (nvidiaFile)
      ./install.yaml
    ];
    control = [./control/schedule.yaml];
    worker = [];
  }