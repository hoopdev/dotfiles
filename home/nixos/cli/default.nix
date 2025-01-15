{ pkgs, ... }:
{
  imports = [
  ];
  home.packages = with pkgs; [
    ollama
  ];
}
