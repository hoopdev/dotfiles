{ inputs, helpers, ... }:
let
  inherit (helpers) nixpkgsConfig gtk4ThemeSilencer;

  ubuntuPkgs = import inputs.nixpkgs {
    system = "x86_64-linux";
    config = nixpkgsConfig;
  };

  mkUbuntu =
    username:
    inputs.home-manager.lib.homeManagerConfiguration {
      pkgs = ubuntuPkgs;
      modules = [
        inputs.stylix.homeModules.stylix
        (import ../lib/stylix.nix { })
        ../hosts/kt-ubuntu/home.nix
        gtk4ThemeSilencer
      ];
      extraSpecialArgs = {
        inherit username inputs;
      };
    };
in
{
  flake.homeConfigurations = {
    "ktaga@kt-ubuntu" = mkUbuntu "ktaga";
    "jovyan@kt-ubuntu" = mkUbuntu "jovyan";
  };
}
