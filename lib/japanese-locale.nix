{ pkgs, ... }:

{
  # Timezone
  time.timeZone = "Asia/Tokyo";

  # Japanese locale
  i18n.defaultLocale = "ja_JP.UTF-8";

  # Japanese input method (Fcitx5 + Mozc)
  i18n.inputMethod = {
    enable = true;
    type = "fcitx5";
    fcitx5.addons = [ pkgs.fcitx5-mozc ];
  };

  # Fonts themselves (HackGen + Noto CJK + emoji) are installed and named by
  # Stylix — see lib/stylix.nix. Stylix's fontconfig target already puts each
  # family's font first in defaultFonts, so all that is added here is the emoji
  # fallback it does not append. Listing the fonts again would only duplicate
  # every entry.
  fonts = {
    fontDir.enable = true;
    fontconfig.defaultFonts = {
      serif = [ "Noto Color Emoji" ];
      sansSerif = [ "Noto Color Emoji" ];
      monospace = [ "Noto Color Emoji" ];
    };
  };
}
