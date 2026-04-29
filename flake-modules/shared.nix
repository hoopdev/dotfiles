# Shared helpers exposed to other flake-parts modules via _module.args.helpers.
#
# Sibling modules pick these up by destructuring `helpers` in their function
# signature, e.g. `{ inputs, helpers, ... }:`.
{ inputs, lib, ... }:
let
  defaultUsername = "ktaga";

  # Auto-discover hosts: every directory under hosts/ that contains a meta.nix.
  # Each meta.nix returns { type, system?, users?, configFrom? } describing how
  # the host should be wired up.
  hostsDir = ../hosts;
  hosts = lib.mapAttrs (name: _: import (hostsDir + "/${name}/meta.nix")) (
    lib.filterAttrs (
      name: kind: kind == "directory" && builtins.pathExists (hostsDir + "/${name}/meta.nix")
    ) (builtins.readDir hostsDir)
  );

  # Shared nixpkgs config — applied via the `nixpkgs.config` module option for
  # NixOS/darwin and passed to `import nixpkgs { config = ...; }` for standalone
  # home-manager and devShells.
  nixpkgsConfig = {
    allowUnfree = true;
  };

  # Silence home-manager 26.05 default-change warning while stateVersion
  # remains 24.05. Drop when stateVersion is bumped.
  gtk4ThemeSilencer =
    { config, lib, ... }:
    {
      gtk.gtk4.theme = lib.mkDefault config.gtk.theme;
    };

  # Wires home-manager into a NixOS/darwin system module. Used by both
  # mkNixosConfiguration and mkDarwinConfiguration.
  mkHomeConfiguration =
    {
      username ? defaultUsername,
      hostname,
      hostPath,
      isNixOS ? false,
    }:
    {
      home-manager = {
        useGlobalPkgs = true;
        useUserPackages = true;
        users.${username} = import hostPath;
        backupFileExtension = "backup";
        extraSpecialArgs = {
          inherit username inputs;
        };
        sharedModules = [ gtk4ThemeSilencer ];
      };
      networking.hostName = hostname;
      users.users.${username}.home = if isNixOS then "/home/${username}" else "/Users/${username}";

      nix = {
        gc = {
          automatic = true;
          options = "--delete-older-than 7d";
        }
        // (
          if isNixOS then
            {
              dates = "weekly";
              persistent = true;
            }
          else
            { }
        );
        settings = {
          max-free = 10737418240; # 10GB
          min-free = 536870912; # 512MB
        };
      };
    };
in
{
  _module.args.helpers = {
    inherit
      defaultUsername
      nixpkgsConfig
      gtk4ThemeSilencer
      mkHomeConfiguration
      hosts
      ;
  };
}
