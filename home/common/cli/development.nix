{
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs.stdenv) isDarwin;
in
{
  home.packages =
    with pkgs;
    [
      # Python development (Python versions managed by uv)
      uv # Fast Python package manager
      ruff # Fast Python linter and formatter

      # deno  # temporarily disabled: deno-2.7.13 / rusty-v8-147.2.1 not yet in
      # cache.nixos.org and local V8 build OOMs on 7.5GB RAM. Re-enable once cached.

      # Container development
      docker

      # Build tools
      gcc
      pkg-config
    ]
    # JavaScript/TypeScript development. On macOS Node comes from Homebrew:
    # the Nix copy shadows /opt/homebrew via /etc/profiles, and its npm global
    # prefix is the read-only nix store, so `npm -g` (e.g. codex self-update)
    # fails. Linux keeps the Nix nodejs (npm globals go to $HOME/.npm-global).
    ++ lib.optionals (!isDarwin) [ nodejs ];
}
