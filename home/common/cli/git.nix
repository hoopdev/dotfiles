{ pkgs, ... }:
{
  programs.git = {
    enable = true;
    lfs.enable = true;
    signing.format = "openpgp";
    settings = {
      user = {
        name = "hoopdev";
        email = "taga@sanken.osaka-u.ac.jp";
      };
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
