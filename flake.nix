{
  description = "A terminal music player for OpenSubsonic/Navidrome servers";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "sonic-tui";
          version = "0.3.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            mpv
          ];

          meta = with pkgs.lib; {
            description = "A terminal music player for OpenSubsonic/Navidrome servers";
            homepage = "https://github.com/yourusername/sonic-tui";
            license = licenses.gpl3;
            maintainers = [ ];
            mainProgram = "sonic-tui";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            pkg-config
            openssl
            mpv
          ];
        };
      }
    );
}
