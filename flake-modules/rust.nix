# Flake packages for the `dev` fleet tool — the Cargo workspace in pkgs/.
#
# Promotes dev-cli / dev-tui / dev-zellij from build blocks buried in a
# home-manager module to first-class outputs, so `nix build .#dev` works and the
# build definition (pkgs/default.nix) is the single source home/mac/dev.nix
# also imports. Makes a future extraction to its own repo a flake-boundary swap.
#
# `dev` is a macOS fleet tool, but the crates build (or, for dev-zellij,
# cross-compile to Wasm) on any system, so we expose them for every system in
# the flake. `nix flake check` only *builds* current-system packages — other
# systems are evaluated, not built — so this needs no extra builders. The lean
# `rust` devShell used to iterate on these lives in per-system.nix.
_: {
  # Attr names here must stay statically known (no `optionalAttrs`/`system`
  # guard at this level): the module system reads them to discover option
  # definitions, and forcing `pkgs` that early recurses through `_module.args`.
  # `pkgs` is referenced only inside the (lazy) package values below.
  perSystem =
    { pkgs, ... }:
    let
      dev = import ../pkgs { inherit pkgs; };
    in
    {
      packages = {
        dev = dev.dev-cli;
        inherit (dev) dev-tui dev-zellij;
      };
    };
}
