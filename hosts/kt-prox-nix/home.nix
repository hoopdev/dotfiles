{
  username,
  inputs,
  ...
}:
{
  home = rec {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.11";
    sessionVariables = {
      EDITOR = "nvim";
      NIXPKGS_ALLOW_UNFREE = 1;
    };
    sessionPath =
      [
      ];
  };

  xdg = {
    enable = true;
    userDirs = {
      extraConfig = {
        desktop = "/home/ktaga/Desktop";
        download = "/home/ktaga/Downloads";
        documents = "/home/ktaga/Documents";
        music = "/home/ktaga/Music";
        videos = "/home/ktaga/Videos";
      };
    };
  };

  programs.home-manager.enable = true;

  imports = [
    ../../home/common/cli
    ../../home/nixos/cli
    #../../home/common/gui
    #../../home/nixos/gui
    inputs.nix-colors.homeManagerModules.default
  ];

  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
    #inputs.hyprpanel.packages.x86_64-linux.default
  ];
}
