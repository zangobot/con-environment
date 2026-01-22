{
  pkgs,
  kubelib ? null, 
  name,
  nfsServer,
  nfsPath,
}:
let
  # --- NAS Configuration ---
  nfsConfig = {
    size = "500Gi";               # K8s doesn't enforce this on NFS, but it's required for the spec
  };

  # Define the raw Kubernetes manifests
  manifests = ''
    apiVersion: v1
    kind: PersistentVolume
    metadata:
      name: ${name}-pv
    spec:
      capacity:
        storage: ${nfsConfig.size}
      volumeMode: Filesystem
      accessModes:
        - ReadOnlyMany          # Safe for vLLM scaling
      persistentVolumeReclaimPolicy: Retain
      storageClassName: manual-models
      mountOptions:
        - nfsvers=4.1
        - noatime               # Improves read performance
      nfs:
        server: ${nfsServer}
        path: ${nfsPath}
    ---
    apiVersion: v1
    kind: PersistentVolumeClaim
    metadata:
      name: ${name}-pvc
      namespace: default
    spec:
      accessModes:
        - ReadOnlyMany
      storageClassName: manual-models
      resources:
        requests:
          storage: ${nfsConfig.size}
      volumeName: ${name}-pv
  '';

in
# This writes the string directly to the output file without any extra formatting/indentation
pkgs.writeText "${name}-pvc.yaml" manifests