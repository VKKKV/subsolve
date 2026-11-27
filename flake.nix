{
  description = "Offline cryptogram solver";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux"];
    eachSystem = f:
      nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    packages = eachSystem (pkgs: {
      default = pkgs.callPackage ./package.nix {};
      subsolve = pkgs.callPackage ./package.nix {};
    });

    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        packages = [
          pkgs.cargo
          pkgs.rustc
          pkgs.clippy
          pkgs.rustfmt
        ];
      };
    });

    formatter = eachSystem (pkgs: pkgs.alejandra);
  };
}
