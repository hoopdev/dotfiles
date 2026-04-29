{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # Python development (Python versions managed by uv)
    uv # Fast Python package manager
    ruff # Fast Python linter and formatter

    # JavaScript/TypeScript development
    nodejs
    # deno  # temporarily disabled: deno-2.7.13 / rusty-v8-147.2.1 not yet in
    # cache.nixos.org and local V8 build OOMs on 7.5GB RAM. Re-enable once cached.

    # Container development
    docker

    # Build tools
    gcc
    pkg-config
  ];
}
