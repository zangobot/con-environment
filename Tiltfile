# Tiltfile - Using nix-flake for image builds
load('ext://helm_remote', 'helm_remote')
load('ext://nix_flake', 'build_flake_image')

allow_k8s_contexts('admin@talos-local')
hostname = os.getenv("HOSTNAME", "localhost")
default_registry('ghcr.io/nbhdai')
update_settings(max_parallel_updates=5)

# ============================================================================
# Secrets and Base Setup
# ============================================================================
local_resource(
    'secrets',
    labels=['setup'],
    deps=['.envhost'],  
    cmd='./setup/scripts/create-secrets.sh',
)

k8s_yaml('./setup/k8/model-proxy.yaml')

k8s_resource('ai-proxy',
    labels=['setup'],
    resource_deps=['secrets'],
)

# ============================================================================
# Workshop Hub - Core System (Built with Nix)
# ============================================================================

# Build Hub image using nix flake
build_flake_image(
    'workshop-hub',
    '.',  # Path to flake.nix (current directory)
    'workshop-hub',  # Output name from flake
    deps=[
        './crates/hub/src',
        './crates/hub/Cargo.toml',
        './Cargo.lock',
    ]
)

# Deploy Hub infrastructure
k8s_yaml('./crates/hub/workshop.yaml')

k8s_resource('workshop-hub',
    port_forwards='8080:8080',
    labels=['hub'],
    resource_deps=['ai-proxy'],
)

# # ============================================================================
# # Workshop dev
# # ============================================================================

# yolo 
k8s_yaml('workshops/yolo-l2/dev.yaml')
docker_build(
    'workshop-yolo-l2-notebook',
    'workshops/yolo-l2/notebook',
    dockerfile='workshops/yolo-l2/notebook/Dockerfile',
    live_update=[
        sync('workshops/yolo-l2/notebook', '/app'),
    ]
)
docker_build(
    'workshop-yolo-l2-verification',
    'workshops/yolo-l2/verification',
    dockerfile='workshops/yolo-l2/verification/Dockerfile',
    live_update=[
        sync('workshops/yolo-l2/verification', '/app'),
    ]
)
k8s_resource(
    'yolo-notebook-dev',
    port_forwards=['8888:8888'],
    labels=['yolo-l2']
)
k8s_resource(
    'challenge-server',
    labels=['yolo-l2']
)

# email
k8s_yaml(
    'workshops/email-indirect/dev.yaml'
)
docker_build(
    'workshop-email-indirect-user',
    'workshops/email-indirect/user',
    dockerfile='workshops/email-indirect/user/Dockerfile',
    live_update=[
        sync('workshops/email-indirect/user', '/app'),
    ]
)

docker_build(
    'workshop-email-indirect-service',
    'workshops/email-indirect/service',
    dockerfile='workshops/email-indirect/service/Dockerfile',
    live_update=[
        sync('workshops/email-indirect/service', '/app'),
    ]
)

k8s_resource(
    'email-client-dev',
    port_forwards=['8090:5000'],
    labels=['email-indirect']
)

k8s_resource(
    'email-service',
    labels=['email-indirect']
)