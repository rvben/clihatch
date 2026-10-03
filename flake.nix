{
  description = "Reproducible CLI scaffolding with clihatch";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        clihatch = pkgs.callPackage ./nix/package.nix { };
        default = clihatch;
      });

      overlays.default = final: _prev: {
        clihatch = final.callPackage ./nix/package.nix { };
      };

      checks = forAllSystems (pkgs: {
        overlay = pkgs.callPackage ./nix/overlay-check.nix {
          overlay = self.overlays.default;
          packageName = "clihatch";
        };
        package = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        generated = pkgs.callPackage ./nix/generated-check.nix {
          clihatch = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            cargo-nextest
            git
            openssh
            nixfmt
          ];
          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
