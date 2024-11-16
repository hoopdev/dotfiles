{
  description = "KT Nix System Flake";

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
    nixos-hardware.url = "github:NixOS/nixos-hardware";
    nix-colors.url = "github:misterio77/nix-colors";
    wezterm = {
      url = "github:wez/wezterm?dir=nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixvim = {
      #url = "github:nix-community/nixvim";
      url = "github:dc-tec/nixvim";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland.url = "github:hyprwm/Hyprland";
    xremap.url = "github:xremap/nix-flake";
  };

  outputs =
    inputs@{
      self,
      nix-darwin,
      nixpkgs,
      nixos-hardware,
      home-manager,
      wezterm,
      hyprland,
      xremap,
      ...
    }:
    let
      # Function for NixOS configuration
      mkNixosConfiguration =
        { hostname, username }:
        nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./hosts/${hostname}/configuration.nix
            home-manager.nixosModules.home-manager
            {
              networking.hostName = hostname;
              users.users.${username}.home = "/home/${username}";
              home-manager.useGlobalPkgs = true;
              home-manager.useUserPackages = true;
              home-manager.users.${username} =
                {
                  pkgs,
                  lib,
                  ...
                }:
                import ./hosts/${hostname}/home.nix {
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
        kt-prox-nix = mkNixosConfiguration {
          hostname = "kt-prox-nix";
          username = "ktaga";
        };
        kt-thinkpad = mkNixosConfiguration {
          hostname = "kt-thinkpad";
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
