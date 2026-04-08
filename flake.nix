{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    zig-overlay.url = "github:mitchellh/zig-overlay";
    zig-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, zig-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default zig-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        z = pkgs.zigpkgs."0.14.0";   # pinned 0.14.0 from the overlay

        darwinDeps = pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs; [
          libiconv
        ]);

        linuxDeps = pkgs.lib.optionals pkgs.stdenv.isLinux (with pkgs; [
          openssl
        ]);
      in
      {
        devShells.default = (pkgs.mkShell.override {
          stdenv = pkgs.llvmPackages_19.stdenv;
        }) {
          packages = with pkgs; [
            rustToolchain
            pkg-config
            z
            llvmPackages_19.llvm
            llvmPackages_19.compiler-rt
            # llvmPackages_19.stdenv
            llvmPackages_19.lld
            # llvmPackages_19.clang
            # llvmPackages_19.libclang
            # llvmPackages_19.compiler-rt
          ];

          buildInputs = with pkgs; [
            openssl sccache
            llvmPackages_19.compiler-rt
            # llvmPackages_19.llvm
            llvmPackages_19.lld
            # llvmPackages_19.clang
            # llvmPackages_19.libclang
          ] ++ darwinDeps ++ linuxDeps;

          # CXX = "${pkgs.llvmPackages_19.clang}/bin/clang++";
          # CFLAGS = "${pkgs.llvmPackages_19.clang}/resource-root/include";
          RUST_BACKTRACE = "1";

          RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
          ASAN_LIB_PATH = "${pkgs.llvmPackages_19.compiler-rt}";
          # CC = "${pkgs.zigpkgs."0.14.0"}/bin/zig cc";
          LD = "${pkgs.llvmPackages_19.lld}/bin/lld";
          # LIBRARY_PATH = pkgs.lib.makeLibraryPath [
          #   pkgs.llvmPackages_19.compiler-rt
          # ];
              # or more precisely:
          shellHook = ''
            export ZIG_LIB_DIRS="${pkgs.lib.makeLibraryPath [
              pkgs.llvmPackages_19.compiler-rt
            ]}"

            export LD="${pkgs.llvmPackages_19.lld}/bin/lld";
            # export CC="${pkgs.zigpkgs."0.14.0"}/bin/zig cc"
            export ZIG_INCLUDE_DIRS="${pkgs.lib.makeSearchPathOutput "dev" "include" [
              pkgs.llvmPackages_19.compiler-rt.dev

            ]}"

          '';

        };
      }
    );
}
