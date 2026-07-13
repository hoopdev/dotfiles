{
  description = "KT Nix System Flake";

  inputs = {
    # Pinned to the last Hydra-green aarch64-darwin eval (2026-07-08 11:07,
    # eval 1826973): later revs (0bb7ec5, 2026-07-08 14:55 onward) ship a
    # cctools/ld64-957.1 that SIGTRAPs (Trace/BPT trap: 5, exit 133) linking
    # some aarch64-darwin binaries (starship, unar/XADMaster, …). This rev's
    # darwin builds are all substitutable from cache.nixos.org. Return to
    # "github:NixOS/nixpkgs/nixpkgs-unstable" once upstream fixes ld64
    # (verify with: nix build --no-link nixpkgs#starship).
    nixpkgs.url = "github:NixOS/nixpkgs/999488783490e5a7bf0b4393f8ddbe7daf10edfe";
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
    wezterm = {
      url = "github:wez/wezterm?dir=nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    dev = {
      # git+file (not path:) so only git-tracked files are hashed — this excludes
      # the multi-GB gitignored target/ dir, whose churn otherwise re-hashed the
      # input and forced a full dev rebuild on every `nh switch`.
      url = "git+file:///Users/ktaga/git/dev";
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
        ./flake-modules/modules.nix
        ./flake-modules/nixos.nix
        ./flake-modules/darwin.nix
        ./flake-modules/home.nix
        ./flake-modules/per-system.nix
        ./flake-modules/dev.nix
      ];
    };
}
