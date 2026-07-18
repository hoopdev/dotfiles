{
  type = "darwin";
  system = "aarch64-darwin";
  primaryUser = "ktaga";
  configFrom = "_shared-mac";
  homeProfiles = [
    "mac"
    "developer"
  ];
  paths = {
    devSource = "/Users/ktaga/git/dev";
  };
}
