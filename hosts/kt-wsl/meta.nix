{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  # WSL intentionally omits desktop-only cache and 1Password GUI profiles.
  systemProfiles = [ "base" ];
  homeProfiles = [
    "nixos-headless"
    "developer"
    "syncthing"
  ];
}
