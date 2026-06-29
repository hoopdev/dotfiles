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
      # Fetch over HTTPS (anonymous, fast), push over SSH (1Password agent).
      url."git@github.com:".pushInsteadOf = "https://github.com/";
    };
  };

  programs.gh = {
    enable = true;
    package = pkgs.gh;
    extensions = [
    ];
  };
}
