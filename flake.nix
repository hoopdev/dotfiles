{
  description = "KT Darwin & home-manager system flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nix-darwin = {
      url = "github:LnL7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-colors.url = "github:misterio77/nix-colors";
    nixvim = {
      #url = "github:nix-community/nixvim";
      url = "github:dc-tec/nixvim";
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
      # Function for NixOS configuration
      mkNixosConfiguration = 
      { hostname, username }:
      nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ./hosts/prox-nix/configuration.nix
          home-manager.nixosModules.home-manager
          {
            networking.hostName = hostname;
            users.users.${username}.home = "/home/${username}";
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.users.${username} =
              { pkgs, lib, ... }:
              import ./hosts/prox-nix/home.nix {
                inherit
                  pkgs
                  lib
                  username
                  inputs
                  ;
              };
          }
        ];
      };


      # Function for macOS configuration
      mkDarwinConfiguration =
        { hostname, username }:
        nix-darwin.lib.darwinSystem {
          system = "aarch64-darwin";
          modules = [
            ./hosts/mac/configuration.nix
            home-manager.darwinModules.home-manager
            {
              networking.hostName = hostname;
              users.users.${username}.home = "/Users/${username}";
              home-manager.useGlobalPkgs = true;
              home-manager.useUserPackages = true;
              home-manager.users.${username} =
                { pkgs, lib, ... }:
                import ./hosts/mac/home.nix {
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
      # Build nixos using flake
      nixosConfigurations = {
        prox-nix = mkNixosConfiguration {
	  hostname = "kt-prox-nix";
	  username = "ktaga";
	};
      };

      # Build darwin using flake
      darwinConfigurations = {
        kt-mac-studio = mkDarwinConfiguration {
          hostname = "kt-mac-studio";
          username = "ktaga";
	};
        kt-mba = mkDarwinConfiguration {
          hostname = "kt-mba";
          username = "ktaga";
	};
      };
    };
}
