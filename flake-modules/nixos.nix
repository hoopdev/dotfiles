{ inputs, helpers, ... }:
let
  inherit (helpers) defaultUsername nixpkgsConfig mkHomeConfiguration;

  mkNixosConfiguration =
    {
      hostname,
      system,
      username ? defaultUsername,
    }:
    inputs.nixpkgs.lib.nixosSystem {
      inherit system;
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
        ../hosts/${hostname}/configuration.nix
        inputs.stylix.nixosModules.stylix
        (import ../lib/stylix.nix { })
        inputs.home-manager.nixosModules.home-manager
        (mkHomeConfiguration {
          inherit username hostname;
          hostPath = ../hosts/${hostname}/home.nix;
          isNixOS = true;
        })
      ];
      specialArgs = {
        inherit inputs;
        inherit (inputs.nixpkgs) lib;
      };
    };
in
{
  flake.nixosConfigurations = {
    kt-proxmox = mkNixosConfiguration {
      hostname = "kt-proxmox";
      system = "x86_64-linux";
    };
    kt-thinkpad = mkNixosConfiguration {
      hostname = "kt-thinkpad";
      system = "x86_64-linux";
    };
    kt-wsl = mkNixosConfiguration {
      hostname = "kt-wsl";
      system = "x86_64-linux";
    };
  };
}
