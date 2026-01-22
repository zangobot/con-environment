{ pkgs, username, token }:

# Validate inputs at build time
if username == "" || token == "" then
  builtins.throw "❌ GITHUB_USERNAME or GHCR_PAT is empty. Did you run with --impure?"
else
  pkgs.runCommand "ghcr-auth.yaml" { 
    nativeBuildInputs = [ pkgs.coreutils ]; 
  } ''
    # Logic runs inside the Nix build sandbox
    AUTH_STRING=$(echo -n "${username}:${token}" | base64 -w 0)

    cat > $out <<EOF
machine:
  registries:
    config:
      ghcr.io:
        auth:
          auth: "$AUTH_STRING"
EOF
  ''