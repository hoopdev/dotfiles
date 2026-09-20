{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  # WSL intentionally omits desktop-only cache and 1Password GUI profiles.
  systemProfiles = [
    "base"
    "headless"
  ];
  homeProfiles = [
    "nixos-headless"
    "developer"
    "syncthing"
  ];
}
