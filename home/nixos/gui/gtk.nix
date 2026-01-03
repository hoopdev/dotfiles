{ pkgs, lib, ... }:

{
  # dconf settings for GNOME/GTK applications
  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  # GTK theming handled by Stylix - these are kept as fallback/override
  gtk = {
    enable = true;

    # Theme, icon, and cursor are managed by Stylix
    # Uncomment below to override Stylix settings:
    # theme = {
    #   name = "Nordic";
    #   package = pkgs.nordic;
    # };

    # iconTheme = {
    #   name = "Papirus-Dark";
    #   package = pkgs.papirus-icon-theme;
    # };

    # cursorTheme = {
    #   name = "Nordzy-cursors";
    #   package = pkgs.nordzy-cursor-theme;
    #   size = 32;
    # };

    # font = {
    #   name = "HackGen Console NF";
    #   size = 11;
    # };

    gtk3.extraConfig = {
      gtk-application-prefer-dark-theme = true;
    };

    gtk4.extraConfig = {
      gtk-application-prefer-dark-theme = true;
    };
  };

  # Pointer cursor is managed by Stylix via stylix.cursor settings
  # home.pointerCursor = {
  #   gtk.enable = true;
  #   x11.enable = true;
  #   name = "Nordzy-cursors";
  #   package = pkgs.nordzy-cursor-theme;
  #   size = 32;
  # };

  # Qt theming handled by Stylix
  # qt = {
  #   enable = true;
  #   platformTheme.name = "gtk";
  #   style.name = "gtk2";
  # };
}
