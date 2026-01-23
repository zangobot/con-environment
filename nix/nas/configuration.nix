{ config, pkgs, inputs, ... }:
  let
    ip = "10.211.0.10";
    control = {
      ip = "10.211.0.20";
      mac = "38:05:25:34:33:04";
      host = "aivControl";
    };
    workers = [
      {
        ip = "10.211.0.21";
        mac = "58:47:ca:7f:54:64";
        host = "aivWorker1";
      }
      {
        ip = "10.211.0.22";
        mac = "58:47:ca:7f:54:64";
        host = "aivWorker2";
      }
      {
        ip = "10.211.0.23";
        mac = "58:47:ca:7f:54:64";
        host = "aivWorker3";
      }
    ];
    inspectorBuild = inputs.self.nixosConfigurations.inspector.config.system.build;
    bootScript = pkgs.writeText "boot.ipxe" ''
      #!ipxe
      dhcp
      echo Starting Inspector Boot...
      kernel tftp://${ip}/bzImage init=${inspectorBuild.toplevel}/init loglevel=4
      initrd tftp://${ip}/initrd
      boot
    '';
    talosImages = import ./talos-image.nix { 
      inherit pkgs; 
      version = "v1.12.1";
      platform = "metal";
      arch = "amd64";
      systemExtensions = [
        "siderolabs/amd-ucode"
        "siderolabs/intel-ucode"
        "siderolabs/nonfree-kmod-nvidia-lts"
        "siderolabs/nvidia-container-toolkit-lts"
      ];
      sha256 = "sha256-xbWnVCIH9JMp9nyBnUKyCZsHafKUtr0ZfOwTqHdlMWU=";

      diskImage = "pxe-assets";
    };
  in
{
  imports =
    [
      ./hardware-configuration.nix
    ];

  # ==========================================
  # 1. Boot & System Basics
  # ==========================================
  
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  networking.hostName = "cluster-control";
  networking.hostId = "8425e349"; 

  # ==========================================
  # 2. ZFS & Storage Configuration
  # ==========================================
  
  boot.supportedFilesystems = [ "zfs" ];
  services.zfs.autoScrub.enable = true;

  fileSystems."/mnt/data" = {
    device = "tank/share";
    fsType = "zfs";
    options = [ "zfsutil" ]; 
  };

  # ==========================================
  # 3. Networking & Firewall
  # ==========================================
  
  networking.interfaces.enp1s0.ipv4.addresses = [{
    address = ip;
    prefixLength = 24;
  }];
  networking.defaultGateway = "10.211.0.1";
  networking.nameservers = [ "1.1.1.1" "8.8.8.8" ];

  services.tailscale = {
    enable = true;
    authKeyFile = "/var/keys/tailscale_key";
    extraUpFlags = [ "--ssh" ];
  };

  networking.firewall = {
    enable = true;
    trustedInterfaces = [ "tailscale0" ];
    
    allowedTCPPorts = [ 
      22   # Local ssh
      53   # DNS (dnsmasq)
      2049 # NFS
    ]; 
    allowedUDPPorts = [ 
      53   # DNS
      67   # DHCP
      69   # TFTP
    ];
  };

  # ==========================================
  # 4. SSH Configuration
  # ==========================================
  
  services.openssh = {
    enable = true;
    openFirewall = false;
    settings = {
      PasswordAuthentication = false;
      PermitRootLogin = "no";
    };
  };

  # ==========================================
  # 5. NFS Server Configuration
  # ==========================================
  
  services.nfs.server = {
    enable = true;
    exports = ''
      /mnt/data 10.211.0.0/24(rw,nohide,insecure,no_subtree_check,no_root_squash)
    '';
  };

  # ==========================================
  # 6. User Account
  # ==========================================

  security.sudo.wheelNeedsPassword = false;

  users.users.admin = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOugqVQLYj89EwYEGthEt0C7OlZh6xRelBdb3LvFDzJb sven@nbhd.ai" 
    ];
  };

  # ==========================================
  # 7. Utilities
  # ==========================================
  environment.systemPackages = with pkgs; [
    vim
    wget
    nano
    talosctl
    kubectl
    git
    htop
    zfs
    zsh
    k9s
    cilium-cli
    hubble
    nmap
    tcpdump
  ];
  # ==========================================
  # 8. Main DHCP & DNS (Pure Dnsmasq PXE)
  # ==========================================

  services.resolved.enable = false;

  services.dnsmasq = {
    enable = true;
    alwaysKeepRunning = true; 
    
    settings = {
      interface = [ "enp1s0" ];
      bind-interfaces = true; 
      log-dhcp = true;

      # DNS
      domain-needed = true;
      bogus-priv = true;
      server = [ "1.1.1.1" "8.8.8.8" ];
      expand-hosts = true;
      domain = "cluster.local";

      # DHCP Subnet
      dhcp-range = [ "10.211.0.50,10.211.0.100,255.255.255.0,24h" ];

      # Options
      dhcp-option = [
        "option:router,10.211.0.1"
        "option:dns-server,${ip}"
      ];

      # Static Hosts
      dhcp-host = [
        "${control.mac},${control.ip},${control.host}"
        "aa:bb:cc:dd:ee:02,10.211.0.21,k8s-worker-01"
        "aa:bb:cc:dd:ee:03,10.211.0.22,k8s-worker-02"
        "aa:bb:cc:dd:ee:03,10.211.0.24,k8s-worker-03"
      ];
      address = [ 
        "/nas/${ip}"
        "/.aiv/${control.ip}"
      ];

      # ==========================================
      # TFTP & PXE Configuration
      # ==========================================
      enable-tftp = true;
      tftp-root = "/var/lib/tftpboot";

      # 1. Tagging: Detect if the client is BIOS (Legacy), UEFI, or iPXE
      dhcp-match = [
        "set:efi-x86_64,option:client-arch,7"
        "set:efi-x86_64,option:client-arch,9"
        "set:ipxe,175" # iPXE sends option 175
      ];

      # 2. Boot Logic (Chainloading)
      # If the client is NOT iPXE yet (!ipxe), send the iPXE bootloader.
      dhcp-boot = [
        "tag:!ipxe,tag:!efi-x86_64,undionly.kpxe"  # Legacy BIOS -> load undionly.kpxe
        "tag:!ipxe,tag:efi-x86_64,ipxe.efi"        # UEFI -> load ipxe.efi
        "tag:ipxe,boot.ipxe"                       # If iPXE is running -> load script
      ];
    };
  };

  # ==========================================
  # 9. PXE Files Setup (The "Plumbing")
  # ==========================================
  systemd.tmpfiles.rules = [
    "d /var/lib/tftpboot 0755 root root -"
    "L+ /var/lib/tftpboot/ipxe.efi - - - - ${pkgs.ipxe}/ipxe.efi"
    "L+ /var/lib/tftpboot/undionly.kpxe - - - - ${pkgs.ipxe}/undionly.kpxe"
    "L+ /var/lib/tftpboot/bzImage - - - - ${inspectorBuild.kernel}/bzImage"
    "L+ /var/lib/tftpboot/initrd - - - - ${inspectorBuild.netbootRamdisk}/initrd"
    "L+ /var/lib/tftpboot/boot.ipxe - - - - ${bootScript}"
  ];
}