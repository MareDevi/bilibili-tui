{
  description = "bilibili-tui - A TUI client for Bilibili";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Static musl OpenSSL
        muslPkgs = pkgs.pkgsCross.musl64;
        opensslMusl = muslPkgs.pkgsStatic.openssl;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "x86_64-unknown-linux-musl" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Build
            pkg-config
            openssl.dev
            openssl.out

            # Runtime
            mpv
            yt-dlp

            # Dev tools
            cargo-dist
            pre-commit
          ];

          nativeBuildInputs = [
            # musl cross toolchain
            muslPkgs.stdenv.cc
          ];

          # OpenSSL for native builds (dynamic linking)
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

          # For musl static linking
          CC_x86_64_unknown_linux_musl = "${muslPkgs.stdenv.cc}/bin/x86_64-unknown-linux-musl-cc";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${muslPkgs.stdenv.cc}/bin/x86_64-unknown-linux-musl-cc";

          # OpenSSL for musl target (static) - target-specific env vars
          X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC = "1";
          X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR = "${opensslMusl.dev}";
          X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_LIB_DIR = "${opensslMusl.out}/lib";
          X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_INCLUDE_DIR = "${opensslMusl.dev}/include";

          shellHook = ''
            echo "bilibili-tui devShell"
            echo "  Rust: $(rustc --version)"
            echo "  Target: x86_64-unknown-linux-musl available"
          '';
        };
      }
    );
}
