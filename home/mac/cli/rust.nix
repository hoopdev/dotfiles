{ pkgs, ... }:
# Rust toolchain for local iteration on the `dev` fleet tool (~/git/dev). The
# dev binaries ship as flake-pinned packages (see dev.nix), but the wrappers
# there prefer a locally cargo-built artifact so a plain `cargo build` / `just
# build` is picked up switchlessly — that needs cargo + rustc on PATH. Also
# feeds the rust_analyzer LSP configured in Neovim (home/common/cli/init.lua).
{
  home.packages = with pkgs; [
    rustc # Rust compiler
    cargo # Rust package manager / build tool
    clippy # Linter
    rustfmt # Formatter
    rust-analyzer # LSP server (used by Neovim)
  ];
}
