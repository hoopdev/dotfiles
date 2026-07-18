{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers)
    nixpkgsConfig
    gtk4ThemeSilencer
    mkHomeProfileModule
    hosts
    ;

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
        let
          paths = meta.paths or { };
          repoPath = paths.repo or null;
          devSource = paths.devSource or null;
          profileNames = (meta.homeProfiles or [ ]) ++ ((meta.userProfiles or { }).${username} or [ ]);
          homeDirectory = "/home/${username}";
          homeBaseModule =
            { lib, ... }:
            {
              home = {
                inherit username homeDirectory;
                stateVersion = meta.homeStateVersion or "24.05";
              };
            }
            // lib.optionalAttrs (repoPath != null || devSource != null) {
              dotfiles.paths =
                { }
                // lib.optionalAttrs (repoPath != null) { repo = repoPath; }
                // lib.optionalAttrs (devSource != null) { inherit devSource; };
            };
        in
        lib.nameValuePair "${username}@${hostname}" (
          inputs.home-manager.lib.homeManagerConfiguration {
            inherit pkgs;
            modules = [
              (mkHomeProfileModule profileNames)
              homeBaseModule
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
