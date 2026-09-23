# Shared development shell (`nix develop` / direnv `use flake`).
#
# The hook runs under bash (both `nix develop` and direnv evaluate it there),
# so it must not try to initialise zsh, starship or zoxide — those come from
# the Home Manager zsh config when you run `nix develop --command zsh`.
{ pkgs, lib, ... }:

let
  # Reuse the CLI tool list from the Home Manager config so the devshell and
  # the login shell agree on what is available. tools.nix is a plain module
  # that only needs pkgs/lib, which is what makes this import possible.
  commonCliTools = import ../home/common/cli/tools.nix { inherit pkgs lib; };
  commonAliases = import ../home/common/cli/shell/aliases.nix;
  cliPackages = commonCliTools.home.packages;

  inherit (pkgs.stdenv.hostPlatform) isLinux;

  # Libraries exposed to unpatched binaries (uv-managed Python, wheels) via
  # nix-ld on NixOS. On other Linux distributions the host loader is used and
  # NIX_LD_LIBRARY_PATH is simply ignored.
  libraries =
    with pkgs;
    [
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
    ]
    ++ lib.optionals isLinux [
      glibc
      stdenv.cc.cc
      stdenv.cc.cc.lib
      libGL
      libx11
      libxext
      libxrender
      libice
      libsm
    ];

  shellPackages =
    with pkgs;
    [
      zsh
      starship
      direnv
      nix-direnv
    ]
    ++ cliPackages;

  aliasCommands = lib.concatStringsSep "\n" (
    lib.mapAttrsToList (name: value: "alias ${name}='${value}'") commonAliases
  );

  commonShellHook = environment: ''
    if [[ $- == *i* ]]; then
      echo "${environment}"
    fi

    # Shell aliases from home/common/cli/shell/aliases.nix (interactive bash only;
    # zsh gets them from Home Manager).
    ${aliasCommands}
  '';

  mkShell =
    {
      environment,
      packages ? [ ],
      shellHook ? "",
    }:
    pkgs.mkShell {
      packages = shellPackages ++ packages;
      # Never replace the shell here: direnv and `nix develop --command`
      # evaluate shellHook too, including non-interactive agent commands.
      shellHook = commonShellHook environment + shellHook;
    };
in
{
  inherit shellPackages mkShell;

  shells.default = mkShell {
    environment = "🚀 Development environment with Python & Nix tools";
    packages =
      with pkgs;
      [
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
        nixfmt
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

        # Task runner
        just

        # Version control tools
        pre-commit
      ]
      ++ libraries;

    shellHook = ''
      # Let nix-ld supply libraries on NixOS without overriding the host
      # loader on other Linux distributions. In particular, putting Nix's
      # glibc in LD_LIBRARY_PATH crashes host binaries such as /bin/sh and
      # the native Claude Code executable on Ubuntu.
      # Prepend rather than assign: on NixOS the system nix-ld list set by
      # programs.nix-ld stays available; on other distros the variable is
      # unset, so nothing changes there.
      ${lib.optionalString isLinux ''
        export NIX_LD_LIBRARY_PATH="${lib.makeLibraryPath libraries}''${NIX_LD_LIBRARY_PATH:+:$NIX_LD_LIBRARY_PATH}"
      ''}

      # uv: prefer its own managed interpreters over whatever python is on
      # PATH. The cache stays at uv's default (~/.cache/uv) so downloads are
      # shared across projects instead of duplicated per checkout.
      export UV_PYTHON_PREFERENCE="managed"
    '';
  };
}
