{
  description = "Word Arena - Multiplayer word game with Rust backend and React frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain from rust-toolchain.toml
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust development
            rustToolchain
            cargo-watch
            cargo-edit
            rust-analyzer
            
            # Node.js development
            nodejs_20
            nodePackages.npm
            nodePackages.pnpm
            nodePackages.typescript
            nodePackages.typescript-language-server
            
            # Build tools
            pkg-config
            
            # System dependencies
            openssl
            sqlite
            
            # Development tools
            git
            jq
            ripgrep
            
            # Optional: Database tools
            sqlite-interactive
            sqlx-cli
          ];

          shellHook = ''
            echo "Word Arena Development Environment"
            echo "===================================="
            echo "Rust version: $(rustc --version)"
            echo "Node version: $(node --version)"
            echo "npm version: $(npm --version)"
            echo ""
            echo "Available commands:"
            echo "  npm run dev           - Start both frontend and backend"
            echo "  npm run dev:frontend  - Start frontend only"
            echo "  npm run dev:backend   - Start backend only"
            echo "  npm run build         - Build everything"
            echo "  npm run test          - Run all tests"
            echo "  cargo watch -x run    - Watch and run Rust backend"
            echo ""
          '';

          # Environment variables
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
          DATABASE_URL = "sqlite://word_arena.db";
        };
      }
    );
}
