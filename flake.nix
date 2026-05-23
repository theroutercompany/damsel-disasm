{
  description = "Damsel reproducible Rust environment and validation checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      forAllSystems =
        function:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          function (
            import nixpkgs {
              inherit system;
            }
          )
        );
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          inherit (pkgs) lib stdenv;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "damsel";
            version = "0.1.0";
            src = self;

            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "-p"
              "damsel-cli"
            ];
            cargoTestFlags = [
              "--workspace"
              "--tests"
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
            ];

            nativeCheckInputs = [
              pkgs.bash
            ];

            buildInputs = lib.optionals stdenv.isDarwin [
              pkgs.libiconv
            ];
          };
        }
      );

      checks = forAllSystems (
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
        in
        {
          damsel = self.packages.${system}.default;

          validation-scripts = pkgs.runCommand "damsel-validation-scripts" { } ''
            cd ${self}
            ${pkgs.bash}/bin/bash -n bin/disassembler-v2-alpha-validate.sh
            ${pkgs.bash}/bin/bash -n bin/disasm-external-compare.sh
            ${pkgs.bash}/bin/bash -n bin/shared-cache-real-smoke.sh
            ${pkgs.bash}/bin/bash -n bin/shared-cache-evidence-package.sh
            ${pkgs.bash}/bin/bash -n bin/shared-cache-evidence-audit.sh
            ${pkgs.ruby}/bin/ruby -e 'require "yaml"; YAML.load_file(".github/workflows/verify.yml"); YAML.load_file(".github/workflows/release.yml")'
            touch "$out"
          '';
        }
      );

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.pkg-config
            pkgs.llvmPackages.llvm
            pkgs.ruby
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          shellHook = ''
            echo "Damsel dev shell"
            echo "  cargo test --workspace"
            echo "  bin/disassembler-v2-alpha-validate.sh --require-external"
          '';
        };
      });

      formatter = forAllSystems (
        pkgs:
        pkgs.writeShellApplication {
          name = "damsel-fmt";
          runtimeInputs = [ pkgs.nixfmt ];
          text = ''
            if [ "$#" -eq 0 ]; then
              exec nixfmt flake.nix
            fi
            exec nixfmt "$@"
          '';
        }
      );
    };
}
