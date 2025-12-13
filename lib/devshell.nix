{ pkgs, lib, ... }:

let
  # Import common CLI tools from home-manager configuration
  commonCliTools = import ../home/common/cli/tools.nix { inherit pkgs; };
  commonAliases = import ../home/common/cli/shell/aliases.nix;

  # Extract packages from home.packages
  cliPackages = commonCliTools.home.packages;

  # Platform detection
  isLinux = pkgs.stdenv.isLinux;
  isDarwin = pkgs.stdenv.isDarwin;

  # Platform-specific libraries
  linuxLibraries = with pkgs; [
    glibc
    stdenv.cc.cc
    stdenv.cc.cc.lib
    libGL
    xorg.libX11
    xorg.libXext
    xorg.libXrender
    xorg.libICE
    xorg.libSM
  ];

  darwinLibraries = with pkgs; [
    # macOS uses system frameworks, minimal additional libs needed
  ];

  # Common libraries (cross-platform)
  commonLibraries = with pkgs; [
    zlib
    glib
    libffi
    openssl
    xz
    bzip2
    ncurses
    readline
    sqlite
    freetype
    fontconfig
    expat
  ];

  # Combined libraries based on platform
  libraries = commonLibraries
    ++ lib.optionals isLinux linuxLibraries
    ++ lib.optionals isDarwin darwinLibraries;
in
{
  # Common shell packages used across all development environments
  shellPackages = with pkgs; [
    # Shell and prompt
    zsh
    starship

    # Development utilities
    direnv
    nix-direnv
  ] ++ cliPackages;

  # Common shell hook for development environments
  shellHook = environment: 
    let
      # Generate alias commands from commonAliases
      aliasCommands = lib.concatStringsSep "\n" (
        lib.mapAttrsToList (name: value: "alias ${name}='${value}'") commonAliases
      );
    in
    ''
      echo "${environment}"
      
      # Set up zsh if available
      if command -v zsh >/dev/null 2>&1; then
        export SHELL=$(command -v zsh)
        
        # Load direnv if available (only in zsh context)
        if [[ -n "$ZSH_VERSION" ]] && command -v direnv >/dev/null 2>&1; then
          eval "$(direnv hook zsh)"
        fi
        
        # Load starship if available (only in zsh context)  
        if [[ -n "$ZSH_VERSION" ]] && command -v starship >/dev/null 2>&1; then
          eval "$(starship init zsh)"
        fi
        
        # Load zoxide if available (only in zsh context)
        if [[ -n "$ZSH_VERSION" ]] && command -v zoxide >/dev/null 2>&1; then
          eval "$(zoxide init zsh)"
        fi
      fi
      
      # Set up shell aliases from home/common/cli/shell/aliases.nix
      ${aliasCommands}
    '';
  
  # Shell configuration with additional packages
  mkShell = { environment, packages ? [], shellHook ? "" }: 
    let
      common = import ./devshell.nix { inherit pkgs lib; };
    in
    pkgs.mkShell {
      packages = common.shellPackages ++ packages;
      shellHook = common.shellHook environment + shellHook + ''
        # Force zsh to be the shell if not already running in zsh
        if [ -z "$ZSH_VERSION" ] && command -v zsh >/dev/null 2>&1; then
          export SHELL="${pkgs.zsh}/bin/zsh"
          exec zsh
        fi
      '';
    };

  # Predefined development shells
  shells = {
    # Unified default development shell with Python support
    default = { environment, devshell }:
      devshell.mkShell {
        environment = "🚀 Development environment with Python & Nix tools";
        packages = with pkgs; [
          # Essential development tools
          git
          curl
          wget

          # Text editors and utilities
          vim
          less
          tree

          # Process management
          htop
          which

          # Nix development (using RFC-style formatter)
          nixfmt-rfc-style
          statix
          deadnix

          # Python runtime
          python313

          # Python package manager
          uv

          # Development tools
          ruff
          mypy
          python3Packages.pytest
          ninja
          meson

          # Build tools
          gcc
          pkg-config

          # Version control tools
          pre-commit
        ] ++ libraries ++ lib.optionals isLinux [
          # Linux-specific packages
          nix-ld
        ];

        shellHook = ''
          # Set library paths (Linux only)
          ${lib.optionalString isLinux ''
            export LD_LIBRARY_PATH="${lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH"
            export NIX_LD_LIBRARY_PATH="${lib.makeLibraryPath libraries}"
          ''}

          # Set up uv environment variables
          export UV_CACHE_DIR="$PWD/.uv-cache"
          export UV_PYTHON_PREFERENCE="managed"

          # Create cache directory if it doesn't exist
          mkdir -p .uv-cache
        '';
      };
  };
}
