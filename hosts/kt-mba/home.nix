{ config, ... }:
let
  # Karabiner-Elements rewrites karabiner.json itself — on version upgrades and
  # on every change made through its GUI — and that write is atomic
  # (temp file + rename), so it replaces whatever sits at the path. A home.file
  # symlink into the read-only Nix store therefore gets clobbered by a plain
  # file, and the profile silently falls back to Karabiner's defaults.
  # Copy the config in as a writable file instead of linking it.
  configSource = ./karabiner.json;
  target = "${config.home.homeDirectory}/.config/karabiner/karabiner.json";
in
{
  # kt-mba specific Karabiner Elements configuration
  home.activation.karabinerConfig = config.lib.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -e "${target}" ] || ! cmp -s ${configSource} "${target}"; then
      $DRY_RUN_CMD mkdir -p "$(dirname "${target}")"
      $DRY_RUN_CMD rm -f "${target}"
      $DRY_RUN_CMD install -m 644 ${configSource} "${target}"
      $DRY_RUN_CMD /bin/launchctl kickstart -k gui/$(id -u)/org.pqrs.karabiner.karabiner_console_user_server 2>/dev/null || true
      echo "Installed Karabiner-Elements configuration"
    fi
  '';
}
