{
  description = "KT Darwin & home-manager system flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nix-darwin.url = "github:LnL7/nix-darwin";
    nix-darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-colors.url = "github:misterio77/nix-colors";
    nixvim = {
      url = "github:nix-community/nixvim";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      self,
      nix-darwin,
      nixpkgs,
      home-manager,
      ...
    }:
    let
      darwinUser = "ktaga";
      darwinHost = builtins.getEnv "DARWIN_HOST";

      mkDarwinSystem =
        { hostname, username }:
        nix-darwin.lib.darwinSystem {
          system = "aarch64-darwin";
          modules = [
            ./config.nix
            home-manager.darwinModules.home-manager
            {
              networking.hostName = hostname;
              users.users.${username}.home = "/Users/${username}";
              home-manager.useGlobalPkgs = true;
              home-manager.useUserPackages = true;
              home-manager.users.${username} =
                { pkgs, lib, ... }:
                import ./hosts/mac.nix {
                  inherit
                    pkgs
                    lib
                    username
                    inputs
                    ;
                };
            }
          ];
          specialArgs = {
            inherit (nixpkgs) lib;
            inherit username;
            inherit inputs;
          };
        };
    in
    {
      # Build darwin flake using:
      # $ darwin-rebuild build --flake .#simple
      darwinConfigurations.${darwinHost} = mkDarwinSystem {
        hostname = darwinHost;
        username = darwinUser;
      };
    };
}
