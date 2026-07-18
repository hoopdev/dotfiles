{
  type = "nixos";
  system = "x86_64-linux";
  primaryUser = "ktaga";
  systemProfiles = [
    "base"
    "onepassword"
  ];
  homeProfiles = [
    "nixos-headless"
    "developer"
    "syncthing"
    "ollama"
  ];
}
