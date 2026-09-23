_:

{
  programs.zsh = {
    enable = true;
    # SSH agent strategy (see also home/common/cli/ssh.nix):
    #   Neither ~/.ssh/config nor config.local sets IdentityAgent anywhere.
    #   Instead, $SSH_AUTH_SOCK is the single source of truth:
    #   - Local shell  → point SSH_AUTH_SOCK at the 1Password agent socket so all
    #                    SSH connections (github.com, remote hosts, …) use local 1Password.
    #   - SSH session  → use only a client-forwarded SSH_AUTH_SOCK, so the
    #                    agent follows the origin machine.
    #   This MUST live in initContent (.zshrc), not loginExtra (.zlogin): zellij
    #   spawns panes as non-login shells, which never source .zlogin — so a
    #   loginExtra override is invisible inside zellij and `git push` there grabs
    #   WezTerm's (empty) mux agent instead of 1Password.
    initContent = ''
      if [[ -z "''${SSH_CONNECTION:-}" ]]; then
        export SSH_AUTH_SOCK="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
      fi

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
