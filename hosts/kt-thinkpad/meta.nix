{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  systemProfiles = [
    "base"
    "onepassword"
    "hyprland-cache"
  ];
  homeProfiles = [
    "nixos-desktop"
    "developer"
    "syncthing"
  ];
}
