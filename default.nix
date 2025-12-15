{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "sonic-tui";
  version = "0.1.0";

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
    libsixel
  ];

  meta = with pkgs.lib; {
    description = "A terminal music player for OpenSubsonic/Navidrome servers";
    homepage = "https://github.com/yourusername/sonic-tui";
    license = licenses.gpl3;
    maintainers = [];
  };
}
