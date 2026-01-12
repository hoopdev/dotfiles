{ lib, pkgs, ... }:

{
  imports = [
    ../../home/mac
  ];

  # kt-mba specific Karabiner Elements configuration
  home.file.".config/karabiner/karabiner.json" = {
    source = ./karabiner.json;
    onChange = ''
      /bin/launchctl kickstart -k gui/$(id -u)/org.pqrs.karabiner.karabiner_console_user_server 2>/dev/null || true
    '';
  };
}
