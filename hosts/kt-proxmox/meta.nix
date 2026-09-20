{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  systemProfiles = [
    "base"
    "onepassword"
    "nvidia"
  ];
  homeProfiles = [
    "nixos-headless"
    "developer"
    "syncthing"
    "ollama"
  ];
}
