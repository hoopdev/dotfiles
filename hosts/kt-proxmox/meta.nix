{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
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
