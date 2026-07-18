{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers)
    defaultUsername
    nixpkgsConfig
    mkHomeConfiguration
    hosts
    profiles
    ;

  nixosHosts = lib.filterAttrs (_: meta: meta.type == "nixos") hosts;

  mkNixosConfiguration =
    hostname: meta:
    let
      username = meta.primaryUser or defaultUsername;
      systemProfiles = meta.systemProfiles or [ "base" ];
      homeProfiles = meta.homeProfiles or [ ];
      paths = meta.paths or { };
    in
    assert lib.all (name: builtins.hasAttr name profiles.nixos) systemProfiles;
    inputs.nixpkgs.lib.nixosSystem {
      inherit (meta) system;
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
      ]
      ++ lib.optionals (lib.elem "onepassword" systemProfiles) [
        {
          dotfiles.onepassword = {
            enable = true;
            polkitPolicyOwners = meta.onepasswordOwners or [ username ];
          };
        }
      ]
      ++ map (name: profiles.nixos.${name}) systemProfiles
      ++ [
        ../hosts/${hostname}/configuration.nix
        inputs.stylix.nixosModules.stylix
        (import ../lib/stylix.nix { })
        inputs.home-manager.nixosModules.home-manager
        (mkHomeConfiguration {
          inherit username;
          inherit hostname;
          hostPath = ../hosts/${hostname}/home.nix;
          isNixOS = true;
          inherit homeProfiles;
          homeStateVersion = meta.homeStateVersion or "24.05";
          repoPath = paths.repo or null;
          devSource = paths.devSource or null;
        })
      ];
      specialArgs = {
        inherit inputs;
        primaryUser = username;
        inherit (inputs.nixpkgs) lib;
      };
    };
in
{
  flake.nixosConfigurations = lib.mapAttrs mkNixosConfiguration nixosHosts;
}
