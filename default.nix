{
  pkgs,
}:

pkgs.rustPlatform.buildRustPackage {
  pname = "notifd";
  version = "0.1.0";

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;

  meta = with pkgs.lib; {
    description = "Notification daemon backend for Quickshell";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "notifd";
  };
}
