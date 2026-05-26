{
  description = "Mute Development Environment";

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
            
            cargo clippy --workspace -- -D warnings

            if [ "''${1:-}" = "--release" ]; then
              cargo build --release --workspace
            else
              cargo build --workspace "$@"
            fi
          '';
        };

        mute-bin = pkgs.writeShellApplication {
          name = "mute";
          runtimeInputs = [ pkgs.git ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            BIN="$ROOT/rust/target/release/mute"
            
            if [ ! -f "$BIN" ]; then
              echo "Error: mute binary not found at $BIN."
              echo "Run 'build --release' first."
              exit 1
            fi
            
            exec "$BIN" "$@"
          '';
        };

        mutex-bin = pkgs.writeShellApplication {
          name = "mutex";
          runtimeInputs = [ pkgs.git ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            BIN="$ROOT/rust/target/release/mutex"
            
            if [ ! -f "$BIN" ]; then
              echo "Error: mutex binary not found at $BIN."
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
          mute-bin
          mutex-bin
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
