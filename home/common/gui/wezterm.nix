_: {
  programs.wezterm = {
    enable = true;

    # Platform differences are resolved by WezTerm at runtime (target_triple)
    # rather than by Nix at eval time, so this exact Lua is also valid on
    # Windows — where it is shipped verbatim by `nix run .#export-dotfiles`.
    extraConfig = ''
      local wezterm = require("wezterm")

      local is_linux = wezterm.target_triple:find("linux") ~= nil

      -- Colors, font (HackGen), font size and opacity all come from Stylix —
      -- do not set them here, or they would silently override the theme.
      return {
        window_padding = {
          left = 10,
          right = 10,
          top = 10,
          bottom = 10,
        },

        use_fancy_tab_bar = false,
        hide_tab_bar_if_only_one_tab = true,
        window_decorations = is_linux and "RESIZE" or "TITLE | RESIZE",

        front_end = "WebGpu",
        enable_wayland = is_linux,
        use_ime = true,
        check_for_updates = false,

        -- Clipboard relies on WezTerm defaults (no override needed):
        --   * SHIFT bypasses an app's mouse capture (e.g. zellij), so
        --     SHIFT+drag selects and copies to the system clipboard — and
        --     SHIFT+wheel scrolls WezTerm natively.
        --   * OSC 52 writes from remote programs (over SSH) are honored,
        --     which is how a remote zellij's copy reaches the Mac clipboard.
        scrollback_lines = 10000,
      }
    '';
  };
}
