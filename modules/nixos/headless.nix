# Shared headless defaults. Remote access, GPU drivers, VM integration and
# user services are host choices; WSL can use this without opening SSH ports.
{ lib, ... }:
{
  services.xserver.enable = lib.mkDefault false;
  services.displayManager.enable = lib.mkDefault false;
  services.printing.enable = lib.mkDefault false;
  services.pipewire.enable = lib.mkDefault false;

  # Keep diagnostic logs across boots without unbounded disk consumption.
  services.journald.extraConfig = ''
    Storage=persistent
    SystemMaxUse=512M
    RuntimeMaxUse=128M
    MaxRetentionSec=30day
  '';
}
