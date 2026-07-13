# Shared home-manager base for HEADLESS NixOS hosts (kt-proxmox, kt-wsl): the
# common CLI stack + Neovim, without the desktop GUI modules that
# home/nixos/default.nix pulls in. Host-specific extras (xdg dirs, ollama, …)
# stay in the per-host home.nix.
{
  username,
  ...
}:
{
  imports = [
    ../common/cli
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };
}
