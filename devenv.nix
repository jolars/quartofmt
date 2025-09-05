{
  pkgs,
  ...
}:

{
  packages = [
    pkgs.go-task
    pkgs.quartoMinimal
    pkgs.wasm-pack
    pkgs.llvmPackages.bintools
  ];

  languages.rust = {
    enable = true;
    # mold.enable = true;
    # channel = "stable";
  };
}
