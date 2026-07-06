{ ... }:

{
  imports = [
    ../common
    ./cli
    ./gui
  ];

  home.sessionVariables = {
    USE_SYMENGINE = "1";
    OLLAMA_HOST = "0.0.0.0";
  };

  home.sessionPath = [
    "/Users/ktaga/.local/bin"
    "/Users/ktaga/.deno/bin"
  ];
}
