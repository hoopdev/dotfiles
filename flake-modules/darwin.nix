{ inputs, helpers, ... }:
let
  inherit (helpers) defaultUsername nixpkgsConfig mkHomeConfiguration;

  mkDarwinConfiguration =
    {
      hostname,
      username ? defaultUsername,
      configPath ? ../hosts/mac/configuration.nix,
      homePath ? ../hosts/mac/home.nix,
    }:
    inputs.nix-darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
        configPath
        inputs.stylix.darwinModules.stylix
        (import ../lib/stylix.nix { darwin = true; })
        inputs.home-manager.darwinModules.home-manager
        (mkHomeConfiguration {
          inherit username hostname;
          hostPath = homePath;
        })
      ];
      specialArgs = {
        inherit inputs;
        inherit (inputs.nixpkgs) lib;
      };
    };
in
{
  flake.darwinConfigurations = {
    kt-mac-studio = mkDarwinConfiguration {
      hostname = "kt-mac-studio";
    };
    kt-mac-mini = mkDarwinConfiguration {
      hostname = "kt-mac-mini";
    };
    kt-mba = mkDarwinConfiguration {
      hostname = "kt-mba";
      configPath = ../hosts/kt-mba/configuration.nix;
      homePath = ../hosts/kt-mba/home.nix;
    };
  };
}
