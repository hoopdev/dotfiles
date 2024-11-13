{ pkgs, ... }:
{
  programs.zsh = {
    enable = true;
    dotDir = ".config/zsh";

    autocd = true;
    enableCompletion = true;
    autosuggestion.enable = true;
    syntaxHighlighting.enable = true;

    shellAliases = import ./aliases.nix;
    initExtra =
      # bash
      ''
        export EDITOR="nvim"
        export NIXPKGS_ALLOW_UNFREE=1
        export PATH="/Users/ktaga/.local/bin:$PATH"
        export OLLAMA_HOST=0.0.0.0
        export OneDrive=/Users/ktaga/Library/CloudStorage/OneDrive-KyotoUniversity
        export USE_SYMENGINE=1
        export PATH="/Users/ktaga/.deno/bin:$PATH"
        export DARWIN_HOST=$(hostname -s)
        export LANG=ja_JP.utf8
        eval "$(/opt/homebrew/bin/brew shellenv)"
      '';
    plugins = [
      {
        name = "fast-syntax-highlighting";
        src = pkgs.zsh-fast-syntax-highlighting.src;
      }
      {
        name = "zsh-completions";
        src = pkgs.zsh-completions.src;
      }
      {
        name = "nix-zsh-completions";
        src = pkgs.nix-zsh-completions.src;
      }
      {
        name = "zsh-nix-shell";
        file = "nix-shell.plugin.zsh";
        src = pkgs.fetchFromGitHub {
          owner = "chisui";
          repo = "zsh-nix-shell";
          rev = "v0.5.0";
          sha256 = "0za4aiwwrlawnia4f29msk822rj9bgcygw6a8a6iikiwzjjz0g91";
        };
      }
    ];

  };
}
