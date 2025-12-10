{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # Python development (Python versions managed by uv)
    uv                    # Fast Python package manager
    ruff                  # Fast Python linter and formatter
    
    # JavaScript/TypeScript development
    nodejs
    deno
    
    # Container development
    docker
    
    # Build tools
    gcc
    pkg-config
  ];
}

