{ pkgs, ... }:
{
  programs.git = {
    enable = true;
    settings = {
      user = {
        name = "hoopdev";
        email = "taga@sanken.osaka-u.ac.jp";
      };
      init = {
        defaultBranch = "main";
      };
    };
  };

  programs.gh = {
    enable = true;
    package = pkgs.gh;
    extensions = [
    ];
  };
}