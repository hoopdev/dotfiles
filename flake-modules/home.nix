{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers) nixpkgsConfig gtk4ThemeSilencer hosts;

  homeHosts = lib.filterAttrs (_: meta: meta.type == "home") hosts;

  mkHostUserPair =
    hostname: meta:
    let
      pkgs = import inputs.nixpkgs {
        inherit (meta) system;
        config = nixpkgsConfig;
      };
      mkUser =
        username:
        lib.nameValuePair "${username}@${hostname}" (
          inputs.home-manager.lib.homeManagerConfiguration {
            inherit pkgs;
            modules = [
              inputs.stylix.homeModules.stylix
              (import ../lib/stylix.nix { })
              ../hosts/${hostname}/home.nix
              gtk4ThemeSilencer
            ];
            extraSpecialArgs = {
              inherit username inputs;
            };
          }
        );
    in
    map mkUser meta.users;

  homeConfigs = lib.listToAttrs (lib.concatLists (lib.mapAttrsToList mkHostUserPair homeHosts));
in
{
  flake.homeConfigurations = homeConfigs;
}
