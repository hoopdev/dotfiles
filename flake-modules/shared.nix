# Shared helpers exposed to other flake-parts modules via _module.args.helpers.
#
# Sibling modules pick these up by destructuring `helpers` in their function
# signature, e.g. `{ inputs, helpers, ... }:`.
{ inputs, lib, ... }:
let
  defaultUsername = "ktaga";
  profiles = import ../lib/profiles.nix;

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

  mkHomeProfileModule =
    profileNames:
    assert lib.all (name: builtins.hasAttr name profiles.home) profileNames;
    {
      imports = map (name: profiles.home.${name}) profileNames;
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
      homeProfiles ? [ ],
      homeStateVersion ? "24.05",
      repoPath ? null,
      devSource ? null,
    }:
    { pkgs, ... }:
    let
      homeDirectory = if isNixOS then "/home/${username}" else "/Users/${username}";
      homeBaseModule =
        { lib, ... }:
        {
          home = {
            inherit username homeDirectory;
            stateVersion = homeStateVersion;
          };
        }
        // lib.optionalAttrs (repoPath != null || devSource != null) {
          dotfiles.paths =
            { }
            // lib.optionalAttrs (repoPath != null) { repo = repoPath; }
            // lib.optionalAttrs (devSource != null) { inherit devSource; };
        };
      backupExistingFile = pkgs.writeShellScript "home-manager-backup-existing-file" ''
        set -eu

        target=$1
        timestamp=$(${pkgs.coreutils}/bin/date -u +%Y%m%dT%H%M%SZ)
        backup="$target.backup.$timestamp"
        suffix=
        index=1

        while [ -e "$backup$suffix" ]; do
          suffix=".$index"
          index=$((index + 1))
        done

        ${pkgs.coreutils}/bin/mv "$target" "$backup$suffix"
      '';
    in
    {
      home-manager = {
        useGlobalPkgs = true;
        useUserPackages = true;
        users.${username} = import hostPath;
        backupCommand = "${backupExistingFile}";
        extraSpecialArgs = {
          inherit username inputs;
        };
        sharedModules = [
          (mkHomeProfileModule homeProfiles)
          homeBaseModule
          gtk4ThemeSilencer
        ];
      };
      networking.hostName = hostname;
      users.users.${username}.home = homeDirectory;

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
      mkHomeProfileModule
      hosts
      profiles
      ;
  };
}
