{ config, ... }:

{
  xdg.configFile."zsh/ssh-agent.sh".source = ./ssh-agent.sh;

  programs.zsh = {
    enable = true;
    # SSH agent strategy (see also home/common/cli/ssh.nix):
    #   Neither ~/.ssh/config nor config.local sets IdentityAgent anywhere.
    #   Instead, $SSH_AUTH_SOCK is the single source of truth:
    #   - Local shell  → local 1Password by default; mux panes inherit their
    #                    parent session's DEV_SSH_AGENT_SOCK, even when empty.
    #   - SSH session  → use only a client-forwarded SSH_AUTH_SOCK, so the
    #                    agent follows the origin machine.
    #   This MUST live in initContent (.zshrc), not loginExtra (.zlogin): zellij
    #   spawns panes as non-login shells, which never source .zlogin — so a
    #   loginExtra override is invisible inside zellij and `git push` there grabs
    #   WezTerm's (empty) mux agent instead of 1Password.
    initContent = ''
      source "${config.xdg.configHome}/zsh/ssh-agent.sh"

      export LANG=ja_JP.utf8
      eval "$(/opt/homebrew/bin/brew shellenv)"
      export PATH="$HOME/.nix-profile/bin:/etc/profiles/per-user/$USER/bin:$PATH"

      # API tokens are not cached on disk any more. Their op:// references are
      # declared in home/mac/default.nix (dotfiles.secrets.env) and resolved on
      # demand by `with-secrets <cmd>` or `secrets-load` — see
      # home/common/cli/onepassword.nix. The old ~/.op-secrets can be deleted.
    '';
  };
}
