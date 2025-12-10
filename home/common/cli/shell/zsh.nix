{ pkgs, config, ... }:
let
  colors = config.colorScheme.palette;
in
{
  programs.zsh = {
    enable = true;
    dotDir = "${config.xdg.configHome}/zsh";

    autocd = true;
    enableCompletion = true;
    autosuggestion = {
      enable = true;
      highlight = "fg=#${colors.base03}";
    };
    syntaxHighlighting = {
      enable = true;
      highlighters = [ "main" "brackets" ];
      styles = {
        # Commands and aliases
        command = "fg=#${colors.base0B}";
        alias = "fg=#${colors.base0B}";
        builtin = "fg=#${colors.base0B}";
        function = "fg=#${colors.base0B}";

        # Arguments and options
        single-hyphen-option = "fg=#${colors.base0A}";
        double-hyphen-option = "fg=#${colors.base0A}";

        # Paths and files
        path = "fg=#${colors.base0C},underline";
        autodirectory = "fg=#${colors.base0C},underline";

        # Strings and quotes
        single-quoted-argument = "fg=#${colors.base0E}";
        double-quoted-argument = "fg=#${colors.base0E}";
        dollar-quoted-argument = "fg=#${colors.base0E}";

        # Variables
        assign = "fg=#${colors.base05}";
        named-fd = "fg=#${colors.base05}";
        numeric-fd = "fg=#${colors.base05}";

        # Errors
        unknown-token = "fg=#${colors.base08}";
        reserved-word = "fg=#${colors.base0D}";
        precommand = "fg=#${colors.base0D},underline";

        # Redirections
        redirection = "fg=#${colors.base09}";

        # Comments
        comment = "fg=#${colors.base03}";

        # Default
        default = "fg=#${colors.base05}";
      };
    };

    shellAliases = import ./aliases.nix;

    plugins = [
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
