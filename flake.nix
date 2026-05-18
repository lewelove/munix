{
  description = "Munix Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        build-cli = pkgs.writeShellApplication {
          name = "build";
          runtimeInputs = [ pkgs.cargo pkgs.rustc pkgs.git pkgs.clippy ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            cd "$ROOT/rust"
            
            cargo clippy -- -D warnings

            if [ "''${1:-}" = "--release" ]; then
              cargo build --release
            else
              cargo build "$@"
            fi
          '';
        };

        munix-bin = pkgs.writeShellApplication {
          name = "munix";
          runtimeInputs = [ pkgs.git ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            BIN="$ROOT/rust/target/release/munix"
            
            if [ ! -f "$BIN" ]; then
              echo "Error: munix binary not found at $BIN."
              echo "Run 'build --release' first."
              exit 1
            fi
            
            exec "$BIN" "$@"
          '';
        };

        devPackages = with pkgs; [
          pkg-config
          openssl
          cargo
          rustc
          rust-analyzer
          rustfmt
          clippy
          build-cli
          munix-bin
          nix
          intermodal
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = devPackages;
          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
          '';
        };
      }
    );
}
