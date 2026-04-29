{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers)
    defaultUsername
    nixpkgsConfig
    mkHomeConfiguration
    hosts
    ;

  darwinHosts = lib.filterAttrs (_: meta: meta.type == "darwin") hosts;

  # If meta sets `configFrom = "x"`, configuration/home are picked up from
  # hosts/x instead of the host's own directory (used for shared mac config).
  hostDir = name: meta: ../hosts/${meta.configFrom or name};

  mkDarwinConfiguration =
    hostname: meta:
    inputs.nix-darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
        (hostDir hostname meta + "/configuration.nix")
        inputs.stylix.darwinModules.stylix
        (import ../lib/stylix.nix { darwin = true; })
        inputs.home-manager.darwinModules.home-manager
        (mkHomeConfiguration {
          username = defaultUsername;
          inherit hostname;
          hostPath = hostDir hostname meta + "/home.nix";
        })
      ];
      specialArgs = {
        inherit inputs;
        inherit (inputs.nixpkgs) lib;
      };
    };
in
{
  flake.darwinConfigurations = lib.mapAttrs mkDarwinConfiguration darwinHosts;
}
