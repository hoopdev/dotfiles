{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers)
    defaultUsername
    nixpkgsConfig
    mkHomeConfiguration
    hosts
    ;

  nixosHosts = lib.filterAttrs (_: meta: meta.type == "nixos") hosts;

  mkNixosConfiguration =
    hostname: meta:
    inputs.nixpkgs.lib.nixosSystem {
      inherit (meta) system;
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
        ../hosts/${hostname}/configuration.nix
        inputs.stylix.nixosModules.stylix
        (import ../lib/stylix.nix { })
        inputs.home-manager.nixosModules.home-manager
        (mkHomeConfiguration {
          username = defaultUsername;
          inherit hostname;
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
  flake.nixosConfigurations = lib.mapAttrs mkNixosConfiguration nixosHosts;
}
