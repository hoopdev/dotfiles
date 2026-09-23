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

  # Silence home-manager's 26.05 default-change warning on hosts whose
  # home.stateVersion predates it. No-op once every meta.nix is >= 26.05;
  # delete it then.
  gtk4ThemeSilencer =
    { config, lib, ... }:
    {
      gtk.gtk4.theme = lib.mkIf (lib.versionOlder config.home.stateVersion "26.05") (
        lib.mkDefault config.gtk.theme
      );
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
      homeStateVersion,
      repoPath ? null,
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
        // lib.optionalAttrs (repoPath != null) {
          dotfiles.paths.repo = repoPath;
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
      # mkMerge + optionalAttrs (not mkIf): the module system rejects a
      # definition for an option that does not exist on the platform even
      # under `mkIf false`, and a plain `//` would shallow-merge `nix`.
      config = lib.mkMerge (
        [
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

            nix.settings = {
              max-free = 10737418240; # 10GB
              min-free = 536870912; # 512MB
            };
          }
        ]
        # Garbage collection runs exactly once a week, at the system level:
        #   NixOS  → programs.nh.clean (`nh clean all`: system + every user's
        #            profiles + store GC). Its module asserts nix.gc.automatic
        #            is off, so the two are never combined.
        #   darwin → nix.gc (nix-darwin has no programs.nh).
        # In both integrated setups Home Manager generations are part of the
        # system generation, so the Home Manager-level nh clean is disabled
        # there (home/common/cli/nh.nix) and only used on standalone hosts.
        ++ lib.optional isNixOS {
          programs.nh = {
            enable = true;
            flake = lib.mkIf (repoPath != null) repoPath;
            clean = {
              enable = true;
              dates = "weekly";
              extraArgs = "--keep-since 7d --keep 5";
            };
          };
        }
        ++ lib.optional (!isNixOS) {
          nix.gc = {
            automatic = true;
            options = "--delete-older-than 7d";
            interval = {
              Weekday = 0;
              Hour = 3;
              Minute = 15;
            };
          };
        }
      );
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
