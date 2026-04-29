# nix-ld for running unpatched dynamic binaries (e.g. uv, Python wheels).
{ pkgs, ... }:

{
  programs.nix-ld = {
    enable = true;
    libraries = with pkgs; [
      stdenv.cc.cc.lib
      zlib
      glib
      libffi
      openssl

      xz
      bzip2
      zstd

      ncurses
      readline

      sqlite

      freetype
      fontconfig
      expat
      libGL
      libx11
      libxext
      libxrender
      libice
      libsm
      libxcursor
      libxrandr
      libxi

      libxkbcommon
      dbus
    ];
  };
}
