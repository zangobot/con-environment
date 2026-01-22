{
  pkgs,
  kubelib,
}:
let
  # Configuration for the NFS Provisioner
  nfsProvisionerValues = {
    nfs = {
      server = "192.168.1.20"; # Your NFS Server IP
      path = "/exports/k8s";   # Your NFS Path
      mountOptions = [
        "nfsvers=4.1"          # FORCE NFSv4
      ];
    };

    storageClass = {
      name = "nfs-client";
      defaultClass = false;    # Set to true if you want this as the default for all PVCs
      
      # Recommended to ensure folders stick around after PVC deletion
      # Change to "Delete" if you want the NAS folder deleted when PVC is deleted
      reclaimPolicy = "Retain"; 
    };
  };

  # Download the Chart
  nfs_chart = kubelib.downloadHelmChart {
    repo = "https://kubernetes-sigs.github.io/nfs-subdir-external-provisioner/";
    chart = "nfs-subdir-external-provisioner";
    version = "4.0.18"; 
    # Replace with actual hash after first run failure
    chartHash = "sha256-0000000000000000000000000000000000000000000="; 
  };

  # Render the Chart
  renderedNfsManifests = kubelib.buildHelmChart {
    name = "nfs-provisioner";
    chart = nfs_chart;
    namespace = "kube-system"; # Usually runs in kube-system or storage
    values = nfsProvisionerValues;
  };

in
pkgs.runCommand "nfs-provisioner.yaml" {} ''
    set -euo pipefail
    
    (
      cat << 'PATCH_START'
cluster:
  inlineManifests:
    - name: nfs-provisioner
      contents: |
        ---
PATCH_START
      sed 's/^/        /' "${renderedNfsManifests}"
      
    ) > "$out"
''