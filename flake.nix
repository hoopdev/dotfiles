{
  description = "KT Nix System Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-darwin = {
      url = "github:LnL7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-hardware.url = "github:NixOS/nixos-hardware";
    stylix = {
      url = "github:nix-community/stylix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    dev = {
      # git+file (not path:) so only git-tracked files are hashed — this excludes
      # the multi-GB gitignored target/ dir, whose churn otherwise re-hashed the
      # input and forced a full dev rebuild on every `nh switch`.
      # Keep the lock portable. Local iteration uses
      # `nix ... --override-input dev path:$HOME/git/dev` (documented in README)
      # while Home Manager wrappers prefer locally built binaries when present.
      # `dev` is private, so use the authenticated Git transport rather than
      # GitHub's anonymous flake API. HTTPS + gh's git-credential helper
      # (git config credential.https://github.com.helper) avoids depending
      # on an SSH agent/key being loaded (e.g. 1Password not running).
      url = "git+https://github.com/hoopdev/dev.git";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-wsl.url = "github:nix-community/nixos-wsl";
    hyprland.url = "github:hyprwm/Hyprland";
    xremap.url = "github:xremap/nix-flake";
    hyprpanel.url = "github:Jas-SinghFSU/HyprPanel";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];
      imports = [
        ./flake-modules/shared.nix
        ./flake-modules/checks.nix
        ./flake-modules/modules.nix
        ./flake-modules/nixos.nix
        ./flake-modules/darwin.nix
        ./flake-modules/home.nix
        ./flake-modules/per-system.nix
        ./flake-modules/export.nix
        ./flake-modules/dev.nix
      ];
    };
}
