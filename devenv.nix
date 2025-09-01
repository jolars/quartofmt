{
  pkgs,
  ...
}:

{
  packages = [
    pkgs.go-task
    pkgs.quartoMinimal
  ];

  languages.rust = {
    enable = true;
    # channel = "stable";
  };
}
