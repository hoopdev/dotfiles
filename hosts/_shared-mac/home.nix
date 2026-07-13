{
  username,
  ...
}:

{
  imports = [
    ../../home/mac
  ];

  home = {
    inherit username;
    homeDirectory = "/Users/${username}";
  };

  home.packages = [
    # Temporarily disabled due to wayland dependency issues on macOS
  ];
}
