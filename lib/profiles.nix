# Named, composable profiles for hosts. Keep the list intentionally small and
# use host metadata to compose them; a new host should mostly be a meta.nix
# entry plus its hardware-specific configuration.
{
  home = {
    cli = ../home/common/cli;
    developer = ../home/common/development.nix;
    syncthing = ../home/common/services/syncthing.nix;
    nixos-desktop = ../home/nixos;
    nixos-headless = ../home/nixos/headless.nix;
    ollama = ../home/nixos/cli;
    mac = ../home/mac;
  };

  nixos = {
    base = ../modules/nixos/default.nix;
    onepassword = ../modules/nixos/onepassword.nix;
    hyprland-cache = ../modules/nixos/hyprland-cache.nix;
  };
}
