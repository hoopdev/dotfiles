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
      url = "github:hoopdev/nixvim";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-wsl.url = "github:nix-community/nixos-wsl";
    hyprland.url = "github:hyprwm/Hyprland";
    xremap.url = "github:xremap/nix-flake";
    hyprpanel.url = "github:Jas-SinghFSU/HyprPanel";
  };

  outputs = inputs@{
    self,
    nixpkgs,
    nix-darwin,
    home-manager,
    nixos-wsl,
    nixos-hardware,
    wezterm,
    hyprland,
    xremap,
    hyprpanel,
    ...
  }: let
      # Common home-manager configuration
      mkHomeConfiguration = { username, hostname, hostPath, isNixOS ? false, extraModules ? [] }: {
        home-manager.useGlobalPkgs = true;
        home-manager.useUserPackages = true;
        home-manager.users.${username} = { pkgs, lib, ... }:
          import hostPath {
            inherit pkgs lib username inputs;
          };
        networking.hostName = hostname;
        users.users.${username}.home = if isNixOS then "/home/${username}" else "/Users/${username}";
      };

      # Common specialArgs
      commonSpecialArgs = {
        inherit (nixpkgs) lib;
        inherit inputs;
      };

      # Function for NixOS configuration
      mkNixosConfiguration = { hostname, username, system }:
        nixpkgs.lib.nixosSystem {
          inherit system;
          modules = [
            ./hosts/${hostname}/configuration.nix
            home-manager.nixosModules.home-manager
            (mkHomeConfiguration {
              inherit username hostname;
              hostPath = ./hosts/${hostname}/home.nix;
              isNixOS = true;
            })
          ] ++ (if hostname == "kt-wsl-nix" then [
            nixos-wsl.nixosModules.wsl
          ] else []);
          specialArgs = commonSpecialArgs // { inherit username; };
        };

      # Function for macOS configuration
      mkDarwinConfiguration = { hostname, username }:
        nix-darwin.lib.darwinSystem {
          system = "aarch64-darwin";
          modules = [
            ./hosts/mac/configuration.nix
            home-manager.darwinModules.home-manager
            (mkHomeConfiguration {
              inherit username hostname;
              hostPath = ./hosts/mac/home.nix;
            })
          ];
          specialArgs = commonSpecialArgs // { inherit username; };
        };

    in
    {
      # Build nixos using flake
      nixosConfigurations = {
        kt-prox-nix = mkNixosConfiguration {
          hostname = "kt-prox-nix";
          username = "ktaga";
          system = "x86_64-linux";
        };
        kt-thinkpad = mkNixosConfiguration {
          hostname = "kt-thinkpad";
          username = "ktaga";
          system = "x86_64-linux";
        };
        kt-wsl-nix = mkNixosConfiguration {
          hostname = "kt-wsl-nix";
          username = "ktaga";
          system = "x86_64-linux";
        };
      };

      # Build darwin using flake
      darwinConfigurations = {
        kt-mac-studio = mkDarwinConfiguration {
          hostname = "kt-mac-studio";
          username = "ktaga";
        };
        kt-mac-mini = mkDarwinConfiguration {
          hostname = "kt-mac-mini";
          username = "ktaga";
        };
        kt-mba = mkDarwinConfiguration {
          hostname = "kt-mba";
          username = "ktaga";
        };
      };
    };
}
