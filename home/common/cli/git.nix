{ pkgs, ... }:
{
  programs.git = {
    enable = true;
    userName = "hoopdev";
    userEmail = "taga.kotaro.62d@st.kyoto-u.ac.jp";
    extraConfig = {
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