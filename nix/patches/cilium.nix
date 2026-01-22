{
  pkgs,
  kubelib,
}:
let
  ciliumValues = {
    ipam = {
      mode = "kubernetes";
    };

    kubeProxyReplacement = true;

    securityContext = {
      capabilities = {
        ciliumAgent = [
          "CHOWN"
          "KILL"
          "NET_ADMIN"
          "NET_RAW"
          "IPC_LOCK"
          "SYS_ADMIN"
          "SYS_RESOURCE"
          "DAC_OVERRIDE"
          "FOWNER"
          "SETGID"
          "SETUID"
        ];
        cleanCiliumState = [
          "NET_ADMIN"
          "SYS_ADMIN"
          "SYS_RESOURCE"
        ];
      };
    };

    cgroup = {
      autoMount.enabled = false;
      hostRoot = "/sys/fs/cgroup";
    };

    k8sServiceHost = "localhost";
    k8sServicePort = 7445;

    dnsproxy = {
      enabled = true;
    };

    hubble = {
      enabled = true;
      relay.enabled = true;
      ui.enabled = true;
    };

    localRedirectPolicy = true;

    bgpControlPlane = {
      enabled = true;
    };

    ingressController = {
      enabled = true;
      default = true;
      loadbalancerMode = "shared";
    };

    tunnelProtocol = "vxlan";

    cni = {
      chainingMode = "none";
      exclusive = true;
    };

    gatewayAPI = {
      enabled = true;
      enableAlpn = true;
      enableAppProtocol = true;
    };

    hostPort = {
      enabled = true;
    };

    nodePort = {
      enabled = true;
    };

    externalIPs = {
      enabled = true;
    };

    loadBalancer = {
      mode = "snat";
      serviceTopology = true;
    };
  };

  cilium_chart = kubelib.downloadHelmChart {
    repo = "https://helm.cilium.io/";
    chart = "cilium";
    version = "v1.18.3";
    chartHash = "sha256-f+3s8+EmXiiqJ5p4dUtpQHWGTYflrO6L9Nj1zMMgh6w=";
  };

  renderedCiliumManifests = kubelib.buildHelmChart {
    name = "cilium";
    chart = cilium_chart;
    namespace = "kube-system";
    values = ciliumValues;
    includeCRDs = true;
  };

in
pkgs.runCommand "cilium.yaml" {} ''
    set -euo pipefail
    
    # Use a subshell to group all output and redirect it once
    (
      cat << 'PATCH_START'
cluster:
  network:
    cni:
      name: none
  proxy:
    disabled: true
  inlineManifests:
    - name: cilium
      contents: |
        ---
PATCH_START
    
      sed 's/^/        /' "${renderedCiliumManifests}"
      
    ) > "$out"
''