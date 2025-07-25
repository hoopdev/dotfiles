{ pkgs, ... }:
{
  # Import shared Mac configuration
  imports = [
    ../mac/configuration.nix
  ];

  # kt-mba specific settings can be added here
}