{ pkgs, ... }:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin;
in
{
  programs.git = {
    enable = true;
    lfs.enable = true;
    # Commit signing with the SSH key held by 1Password: the signer program is
    # wired here, the key is per machine. To turn it on, add to
    # ~/.config/git/local:
    #   [user]   signingkey = ssh-ed25519 AAAA…   (public key of the 1Password item)
    #   [commit] gpgsign = true
    #   [tag]    gpgsign = true
    # Linux resolves `op-ssh-sign` from PATH, i.e. only on hosts with the
    # 1Password GUI (`onepassword` system profile); headless hosts just leave
    # signing off.
    signing = {
      format = "ssh";
      signer =
        if isDarwin then "/Applications/1Password.app/Contents/MacOS/op-ssh-sign" else "op-ssh-sign";
    };
    # user.name/email live here to keep identity out of the public store.
    # Commits fail if this file is absent — create it on each new machine.
    includes = [
      { path = "~/.config/git/local"; }
    ];
    settings = {
      init = {
        defaultBranch = "main";
      };
      # Fetch and push both over HTTPS; `gh auth git-credential` (wired up by
      # programs.gh below) supplies the token. Previously push was rewritten to
      # git@github.com: via url.pushInsteadOf, but that depends on the
      # 1Password SSH agent being up, which silently breaks push with
      # "Permission denied (publickey)" on machines where it isn't.
    };
  };

  programs.gh = {
    enable = true;
    package = pkgs.gh;
    extensions = [
    ];
  };
}
