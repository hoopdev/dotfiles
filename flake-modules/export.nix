# Export portable config artifacts into the Chezmoi source tree.
#
# Some configs must be *generated* rather than hand-written, because their
# values come from Nix/Stylix and cannot be expressed as a static file:
# starship's `format` interpolates the base16 palette, and WezTerm's colors
# come from Stylix's `programs.wezterm.settings`/`colorSchemes`. Hand-copying
# them into chezmoi/ is what made those copies drift.
#
# So Nix stays the single source and this exports the *rendered* artifacts —
# which are pure data (verified free of /nix/store paths) and therefore usable
# on machines without Nix. Windows consumes them via Chezmoi.
#
# Usage: nix run .#export-dotfiles   (from the repo root)
{ inputs, helpers, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      username = helpers.defaultUsername;

      # A dedicated home-manager evaluation whose only purpose is to render the
      # artifacts. It is deliberately NOT one of the real hosts: the output must
      # not depend on which machine runs the export, and it carries the Windows
      # flavour (system glyph) rather than the exporting host's.
      portable = inputs.home-manager.lib.homeManagerConfiguration {
        inherit pkgs;
        modules = [
          inputs.stylix.homeModules.stylix
          (import ../lib/stylix.nix { })
          helpers.gtk4ThemeSilencer
          ../home/common/cli/shell/starship.nix
          ../home/common/gui/wezterm.nix
          {
            home = {
              inherit username;
              homeDirectory =
                if pkgs.stdenv.hostPlatform.isDarwin then "/Users/${username}" else "/home/${username}";
              stateVersion = "24.05";
            };

            # The one value that is genuinely target-specific: the exported
            # prompt is for Windows, not for the host doing the exporting.
            programs.starship.systemLogo = "";
          }
        ];
        extraSpecialArgs = { inherit username inputs; };
      };

      inherit (portable.config.xdg) configFile;

      # home-manager keys starship's file by an absolute path
      # (home.file."$XDG_CONFIG_HOME/starship.toml"), so render the settings the
      # same way it does rather than fishing that entry out by name.
      starshipToml =
        (pkgs.formats.toml { }).generate "starship.toml"
          portable.config.programs.starship.settings;

      weztermLua = configFile."wezterm/wezterm.lua".source;
      weztermColors = configFile."wezterm/colors/stylix.toml".source;
      neovimInit = ../home/common/cli/init.lua;

      banner = "Generated from the Nix config by: nix run .#export-dotfiles — do not edit by hand.";
    in
    {
      packages.export-dotfiles = pkgs.writeShellApplication {
        name = "export-dotfiles";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          root=''${1:-$PWD}

          if [ ! -e "$root/flake.nix" ] || [ ! -e "$root/.chezmoiroot" ]; then
            echo "error: $root is not the dotfiles repo root" >&2
            exit 1
          fi

          cfg="$root/chezmoi/dot_config"
          mkdir -p "$cfg/wezterm/colors" "$cfg/nvim"

          # starship / wezterm carry no comment-safe header of their own, so
          # prepend one; TOML and Lua both accept the respective comment form.
          { echo "# ${banner}"; cat ${starshipToml}; } > "$cfg/readonly_starship.toml"
          { echo "# ${banner}"; cat ${weztermColors}; } > "$cfg/wezterm/colors/stylix.toml"
          { echo "-- ${banner}"; cat ${weztermLua}; } > "$cfg/wezterm/wezterm.lua"

          # init.lua is already a plain portable file — copy it as-is. (The
          # activation hook in home/common/cli/neovim.nix does the same on every
          # rebuild; both copy the same source, so they cannot disagree.)
          cp -f ${neovimInit} "$cfg/nvim/init.lua"

          chmod u+w \
            "$cfg/readonly_starship.toml" \
            "$cfg/wezterm/wezterm.lua" \
            "$cfg/wezterm/colors/stylix.toml" \
            "$cfg/nvim/init.lua"

          echo "exported -> chezmoi/dot_config/{readonly_starship.toml,wezterm/,nvim/init.lua}"
        '';
      };
    };
}
