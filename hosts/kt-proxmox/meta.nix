{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  homeStateVersion = "24.05";
  systemProfiles = [
    "base"
    "headless"
    "nvidia"
  ];
  homeProfiles = [
    "nixos-headless"
    "developer"
    "syncthing"
    "ollama"
  ];
}
