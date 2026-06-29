{ pkgs, lib, ... }:
{
  home.packages = with pkgs; [
    bat
    cloudflared
    eza
    fd
    fx
    fzf
    difftastic
    dust
    procs
    bottom
    ghq
    httpie
    imagemagick
    jq
    zoxide
    unar
    unrar
    unzip
    zip
    zellij
    gotop
    yazi
    ripgrep
    rclone
    rsync
    syncthing
    hackgen-nf-font
    nixfmt
    # quarto  # temporarily disabled: bundles deno-2.7.13 / rusty-v8-147.2.1
    # not yet in cache.nixos.org; local V8 build OOMs on 7.5GB RAM. Re-enable once cached.
    fastfetch
    _1password-cli
    lua-language-server
    pyright
    ruff
    tree-sitter
    lsof
    trash-cli
  ];
  services.syncthing = {
    enable = true;
  };
  programs.zoxide = {
    enable = true;
    package = pkgs.zoxide;
    enableNushellIntegration = true;
    enableZshIntegration = true;
  };
  programs.zellij = {
    enable = true;
    package = pkgs.zellij;
    enableZshIntegration = false;
    settings = {
      # Keep zellij's mouse mode on: click focuses a pane, the wheel scrolls
      # that pane's scrollback. To copy with the mouse, hold SHIFT while
      # dragging — that bypasses zellij and uses WezTerm's native selection,
      # which copies to the system clipboard and behaves identically over SSH
      # (WezTerm copies the rendered screen, no OSC 52 / remote helper needed).
      mouse_mode = true;
      # Auto-copy when a zellij copy-mode / mouse selection is released.
      copy_on_select = true;
    }
    # macOS local sessions: pipe zellij copy-mode selections to pbcopy.
    # On Linux (SSH targets) copy_command is left unset so zellij emits OSC 52
    # instead — the only clipboard mechanism that survives an SSH hop (it is
    # forwarded up to the connecting terminal, i.e. WezTerm on the Mac).
    // lib.optionalAttrs pkgs.stdenv.isDarwin {
      copy_command = "pbcopy";
    };
  };
}
