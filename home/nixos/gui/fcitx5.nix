{ ... }:
{
  # Disable Fcitx5's Emacs-style keybindings that interfere with terminal Ctrl shortcuts
  # (Ctrl+A, Ctrl+E, Ctrl+F, Ctrl+B, etc.)
  xdg.configFile."fcitx5/conf/emacs.conf".text = ''
    # Disable Emacs frontend shortcuts
    Enabled=False
  '';
}
