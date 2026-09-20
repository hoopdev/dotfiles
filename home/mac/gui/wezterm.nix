{ pkgs, ... }:
{
  # WezTerm itself comes from the `wezterm@nightly` Homebrew cask declared in
  # hosts/_shared-mac/configuration.nix; a Nix-installed WezTerm.app would show
  # up in ~/Applications/Home Manager Apps as a second, competing copy.
  # Home Manager still owns ~/.config/wezterm — that is what carries the Stylix
  # colors, font and opacity — so the module stays enabled and only its package
  # is stubbed out.
  programs.wezterm = {
    package = pkgs.emptyDirectory;
    # Both integrations `source ${package}/etc/profile.d/wezterm.sh`, which the
    # stub does not ship; leaving them on would error on every shell startup.
    enableBashIntegration = false;
    enableZshIntegration = false;
  };
}
