# Japanese input method (Fcitx5 + Mozc). Desktop-only: import it alongside
# lib/japanese-locale.nix on hosts with a graphical session. Headless hosts
# must not import it, or they pull in fcitx5 and Mozc for nothing.
{ pkgs, ... }:

{
  i18n.inputMethod = {
    enable = true;
    type = "fcitx5";
    fcitx5.addons = [ pkgs.fcitx5-mozc ];
  };
}
