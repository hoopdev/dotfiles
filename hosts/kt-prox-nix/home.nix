{
  pkgs,
  lib,
  username,
  inputs,
  ...
}:
{
  home = rec {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  programs.home-manager.enable = true;
  #programs.kitty.enable = true;
  imports = [
    ../../home/common/cli
    ../../home/common/gui
    ../../home/nixos/gui
    inputs.nix-colors.homeManagerModules.default
    #inputs.nixvim.homeManagerModules.nixvim
  ];
  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
  ];

  #wayland.windowManager.hyprland.settings = {
  #  "$mod" = "SUPER";
  #  bind =
  #    [
  #      "$mod, F, exec, firefox"
  #      ", Print, exec, grimblast copy area"
  #    ]
  #    ++ (
  #      # workspaces
  #      # binds $mod + [shift +] {1..9} to [move to] workspace {1..9}
  #      builtins.concatLists (
  #        builtins.genList (
  #          i:
  #          let
  #            ws = i + 1;
  #          in
  #          [
  #            "$mod, code:1${toString i}, workspace, ${toString ws}"
  #            "$mod SHIFT, code:1${toString i}, movetoworkspace, ${toString ws}"
  #          ]
  #        ) 9
  #      )
  #    );
  #};
}
