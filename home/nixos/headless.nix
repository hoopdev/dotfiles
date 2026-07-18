# Shared home-manager base for HEADLESS NixOS hosts (kt-proxmox, kt-wsl): the
# common CLI stack + Neovim, without the desktop GUI modules that
# home/nixos/default.nix pulls in. Host-specific extras (xdg dirs, ollama, …)
# stay in the per-host home.nix.
_: {
  imports = [
    ../common/cli
  ];

}
