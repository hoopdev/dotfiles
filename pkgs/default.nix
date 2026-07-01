# Build definitions for the `dev` fleet tool (Cargo workspace in this dir).
#
# Single source of truth for the three artifacts, imported from two places:
#   - flake-modules/rust.nix — exposes them as flake `packages` (`nix build .#dev`)
#   - home/mac/dev.nix        — wraps `dev` with runtime env and installs the rest
#
# All three take `pkgs` and derive their source from `./.` (this dir). Inside a
# flake `./.` is the git-tracked store copy of `pkgs/`, so `target/` (gitignored)
# is never copied in.
{ pkgs }:
let
  inherit (pkgs) lib;
  src = lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;
in
rec {
  # `dev` — the CLI (bin `dev`). dev-core pulls git2 with vendored-libgit2, so
  # the build needs cmake to compile libgit2 from source.
  dev-cli = pkgs.rustPlatform.buildRustPackage {
    pname = "dev-cli";
    version = "0.1.0";
    inherit src cargoLock;
    cargoBuildFlags = [
      "--package"
      "dev-cli"
    ];
    cargoTestFlags = [
      "--package"
      "dev-cli"
    ];
    nativeBuildInputs = [ pkgs.cmake ];
  };

  # `dev tui` — live fleet TUI (ratatui). notify links kqueue on macOS, so no
  # extra inputs are needed here.
  dev-tui = pkgs.rustPlatform.buildRustPackage {
    pname = "dev-tui";
    version = "0.1.0";
    inherit src cargoLock;
    cargoBuildFlags = [
      "--package"
      "dev-tui"
    ];
    cargoTestFlags = [
      "--package"
      "dev-tui"
    ];
  };

  # `dev board` — Zellij task-board plugin (Wasm cdylib). Built with
  # pkgsCross.wasi32 (wasm32-unknown-wasi) using lld, matching nixpkgs
  # zellijPlugins. The build output is a directory; the second derivation
  # extracts the single `.wasm` so consumers get a plain file.
  dev-zellij =
    let
      pkgs' = pkgs.pkgsCross.wasi32;
      unwrapped = pkgs'.rustPlatform.buildRustPackage {
        pname = "dev-zellij";
        version = "0.1.0";
        inherit src cargoLock;
        cargoBuildFlags = [
          "--package"
          "dev-zellij"
        ];
        doCheck = false;
        nativeBuildInputs = [ pkgs'.lld ];
        env.RUSTFLAGS = " -C linker=wasm-ld";
      };
    in
    pkgs.stdenvNoCC.mkDerivation {
      name = "dev-zellij.wasm";
      src = unwrapped;
      dontUnpack = true;
      buildPhase = ''
        cp "$(find "$src" -name '*.wasm')" "$out"
      '';
    };
}
