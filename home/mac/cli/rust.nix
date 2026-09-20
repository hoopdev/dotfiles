{ pkgs, ... }:
# Rust development tools and the Neovim language server.
{
  home.packages = with pkgs; [
    rustc # Rust compiler
    cargo # Rust package manager / build tool
    clippy # Linter
    rustfmt # Formatter
    rust-analyzer # LSP server (used by Neovim)
  ];
}
