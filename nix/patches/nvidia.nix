{
  pkgs,
  kubelib,
}:
let
  # --- Configuration ---
  devicePluginValues = {
    # If using Talos or a custom path, you might need to adjust these
    # but defaults usually work for standard setups.
    gfd = {
      enabled = true; # GPU Feature Discovery (labels nodes with GPU model/memory)
    };
  };

  # Download the NVIDIA Device Plugin Chart
  nvidia_chart = kubelib.downloadHelmChart {
    repo = "https://nvidia.github.io/k8s-device-plugin";
    chart = "nvidia-device-plugin";
    version = "0.17.0"; 
    chartHash = "sha256-0000000000000000000000000000000000000000000="; # Run once to get hash
  };

  # Render the Chart
  renderedNvidiaManifests = kubelib.buildHelmChart {
    name = "nvidia-device-plugin";
    chart = nvidia_chart;
    namespace = "kube-system";
    values = devicePluginValues;
  };

in
pkgs.runCommand "nvidia-plugin.yaml" {} ''
    set -euo pipefail
    
    (
      cat << 'PATCH_START'
cluster:
  inlineManifests:
    - name: nvidia-device-plugin
      contents: |
        ---
PATCH_START
    
      sed 's/^/        /' "${renderedNvidiaManifests}"
      
    ) > "$out"
''