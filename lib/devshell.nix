{ pkgs, lib, ... }:

{
  # Common shell packages used across all development environments
  shellPackages = with pkgs; [
    # Shell and prompt
    zsh
    starship
    
    # CLI tools (matching aliases.nix)
    bat          # cat replacement
    eza          # ls replacement  
    procs        # ps replacement
    dust         # du replacement
    difftastic   # diff replacement
    zoxide       # cd replacement
    
    # File management
    yazi         # Terminal file manager
    
    # Development utilities
    direnv
    nix-direnv
    
    # Session management
    zellij
  ];

  # Common shell hook for development environments
  shellHook = environment: ''
    echo "${environment}"
    
    # Set up zsh if available
    if command -v zsh >/dev/null 2>&1; then
      export SHELL=$(which zsh)
      
      # Load direnv if available
      if command -v direnv >/dev/null 2>&1; then
        eval "$(direnv hook zsh)"
      fi
      
      # Load starship if available
      if command -v starship >/dev/null 2>&1; then
        eval "$(starship init zsh)"
      fi
      
      # Load zoxide if available
      if command -v zoxide >/dev/null 2>&1; then
        eval "$(zoxide init zsh)"
      fi
    fi
    
    # Set up shell aliases (matching aliases.nix)
    alias rm='rm -i'
    alias cp='cp -i'
    alias cd='z'
    alias cat='bat'
    alias ps='procs'
    alias du='dust'
    alias diff='difft'
    alias ls='eza --icons always --classify always'
    alias la='eza --icons always --classify always --all'
    alias ll='eza --icons always --long --all --git'
    alias lt='eza --icons always --classify always --tree'
    alias vim='nvim'
    alias zj='zellij'
    alias yz='yazi'
    
    # Auto-switch to zsh if we're not already in it
    if [ "$SHELL" != "$(which zsh)" ] && command -v zsh >/dev/null 2>&1; then
      exec zsh
    fi
  '';
  
  # Shell configuration with additional packages
  mkShell = { environment, packages ? [], shellHook ? "" }: 
    let
      common = import ./devshell.nix { inherit pkgs lib; };
    in
    pkgs.mkShell {
      packages = common.shellPackages ++ packages;
      shellHook = common.shellHook environment + shellHook;
    };

  # Predefined development shells
  shells = {
    # Default development shell
    default = { environment, devshell }: devshell.mkShell {
      environment = "🏠 Dotfiles development environment";
      packages = with pkgs; [
        # Nix development
        nixpkgs-fmt
        statix
        deadnix
        
        # Git and version control
        git
        pre-commit
      ];
    };

    # Python development shell
    python = { environment, devshell }:
      let
        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;
        libraries = with pkgs; [
          # 基本的なライブラリ
          glibc
          stdenv.cc.cc
          stdenv.cc.cc.lib  # libstdc++を含む
          
          # Python関連ライブラリ
          zlib
          glib
          libffi
          openssl
          xz
          bzip2
          ncurses
          readline
          sqlite
          
          # グラフィックス・UI関連
          libGL
          xorg.libX11
          xorg.libXext
          xorg.libXrender
          xorg.libICE
          xorg.libSM
          freetype
          fontconfig
          expat
        ];
      in
      devshell.mkShell {
        environment = "🐍 Python development environment with uv";
        packages = with pkgs; [
          # Python runtime
          python313
          
          # Python package manager
          uv
          
          # Development tools
          ruff          # Fast Python linter and formatter
          mypy          # Static type checker
          pkgs.python3Packages.pytest        # Testing framework
          ninja
          meson
          
          # Build tools
          gcc
          pkg-config
          
          # Version control
          git
          pre-commit
        ] ++ libraries ++ pkgs.lib.optionals isLinux [
          # Linux-specific packages for manylinux compatibility
          nix-ld
        ] ++ pkgs.lib.optionals isDarwin [
          # macOS-specific packages
        ];
        
        shellHook = ''
          # LD_LIBRARY_PATHを設定
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH"
          export NIX_LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libraries}"
          
          # Set up uv environment variables
          export UV_CACHE_DIR="$PWD/.uv-cache"
          export UV_PYTHON_PREFERENCE="managed"
          
          # Create cache directory if it doesn't exist
          mkdir -p .uv-cache
          
          # デバッグ情報
          echo "libstdc++ location:"
          find ${pkgs.stdenv.cc.cc.lib}/lib -name "libstdc++.so*" 2>/dev/null | head -5
          
          echo "環境が更新されました。'uv sync'を実行して依存関係を再インストールすることをお勧めします。"
        '';
      };
  };
}