{ pkgs, ... }:
{
  programs.git = {
    enable = true;
    lfs.enable = true;
    signing.format = "openpgp";
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
