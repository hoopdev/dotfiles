{ pkgs, lib, username, ... }:

{
  home.packages = with pkgs; [
      pkgs.git
      pkgs.neovim
      pkgs.nushell
      pkgs.starship
      pkgs.zoxide
      pkgs.vscode
  ];

  # ユーザー情報
  home.username = username;
  home.homeDirectory = "/Users/${username}";

  # home-managerのバージョン
  home.stateVersion = "24.05";

  # home-managerの有効化
  programs.home-manager.enable = true;

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
}
