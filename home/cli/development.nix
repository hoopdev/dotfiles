
{ pkgs, ... }:
{
  home.packages = with pkgs; [
    uv
    deno
    docker
  ];
}