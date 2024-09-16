{
  description = "Only Provide the support of qemu";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-qemu7.url = "https://github.com/NixOS/nixpkgs/archive/7cf5ccf1cdb2ba5f08f0ac29fc3d04b0b59a07e4.tar.gz";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-qemu7, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ 
          (import rust-overlay)
          (self: super: {
            # ref: https://github.com/the-nix-way/dev-templates
            rust-toolchain =
              let
                rust = super.rust-bin;
              in
              if builtins.pathExists ./rust-toolchain.toml then
                rust.fromRustupToolchainFile ./rust-toolchain.toml
              else if builtins.pathExists ./rust-toolchain then
                rust.fromRustupToolchainFile ./rust-toolchain
              else
                rust.nightly.latest.default;
                # The rust-toolchain when i make this file, which maybe change
                # (rust.nightly.latest.override {
                #   extensions = [ "rust-src" "llvm-tools-preview" "rustfmt" "clippy" ];
                #   targets = [ "x86_64-unknown-none" "riscv64gc-unknown-none-elf" "aarch64-unknown-none-softfloat" ];
                # });
            qemu7 = self.callPackage "${nixpkgs-qemu7}/pkgs/applications/virtualization/qemu" {
              inherit (self.darwin.apple_sdk.frameworks) CoreServices Cocoa Hypervisor;
              inherit (self.darwin.stubs) rez setfile;
              inherit (self.darwin) sigtool;
              # Reduces the number of qemu source files from ~10000 to ~3619 source files.
              hostCpuTargets = ["riscv64-softmmu" "riscv32-softmmu" "x86_64-softmmu" "aarch64-softmmu" ];
            };
            x86_64-linux-musl-cross = fetchTarball {
              url = "https://musl.cc/x86_64-linux-musl-cross.tgz";
              sha256 = "172zrq1y4pbb2rpcw3swkvmi95bsqq1z6hfqvkyd9wrzv6rwm9jw";
            };
            aarch64-linux-musl-cross = fetchTarball {
              url = "https://musl.cc/aarch64-linux-musl-cross.tgz";
              sha256 = "05cwryhr88sjmwykha5xvfy4vcrvwaz92r9an7n5bsyzlwwk0wpn";
            };
            riscv64-linux-musl-cross = fetchTarball {
              url = "https://musl.cc/riscv64-linux-musl-cross.tgz";
              sha256 = "119y1y3jwpa52jym3mxr9c2by5wjb4pr6afzvkq7s0dp75m5lzvb";
            };
          })
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells = {
          # qemu = pkgs.mkShell {
          #   buildInputs = with pkgs; [
          #     vim
          #     # exa
          #     fd
          #     zlib
          #     qemu
          #   ];

          #   shellHook = ''
          #     alias ls=exa
          #     alias find=fd

          #     # Change the mirror of rust
          #     export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
          #     export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup

          #     unset OBJCOPY # Avoiding Overlay
          #     export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib" # nixpkgs@52447
          #     export LD_LIBRARY_PATH="${pkgs.zlib}/lib:$LD_LIBRARY_PATH" # nixpkgs@92946
          #     
          #     export PATH=$PATH:$(realpath .)/.toolchain/aarch64-linux-musl-cross/bin:$(realpath .)/.toolchain/riscv64-linux-musl-cross/bin:$(realpath .)/.toolchain/x86_64-linux-musl-cross/bin/
          #   '';
          # };
          default = pkgs.mkShell {
            buildInputs = (with pkgs;[
              gnumake
              # Basic
              openssl
              pkg-config
              fd
              # Development tools
              ripgrep
              fzf
              zellij
              # Rust Configuraiton  
              zlib
              rustup
              cargo-binutils
              rust-toolchain
            ]) ++ [
              # Overlays part
              pkgs.qemu
            ];

            # nativeBuildInputs = with pkgs; [
            #   llvmPackages.libclang
            #   llvmPackages.libcxxClang
            #   clang
            # ];
            # LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"; # nixpkgs@52447
            # BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.clang}/include"; # https://github.com/NixOS/nixpkgs/issues/52447#issuecomment-853429315

            shellHook = ''
              alias find=fd
              export SHELL=zsh

              # Change the mirror of rust
              export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
              export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup

              unset OBJCOPY # Avoiding Overlay
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib" # nixpkgs@52447
              export LD_LIBRARY_PATH="${pkgs.zlib}/lib:$LD_LIBRARY_PATH" # nixpkgs@92946
              
              export PATH=$PATH:${pkgs.aarch64-linux-musl-cross}/bin:${pkgs.riscv64-linux-musl-cross}/bin:${pkgs.x86_64-linux-musl-cross}/bin
            '';
          };
        };
      }
    );
}

