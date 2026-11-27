{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "subsolve";
  version = "0.2.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "Offline substitution and Vigenère cipher solver for CTF use";
    homepage = "https://git.k174.dev/k/subsolve";
    license = lib.licenses.gpl3Plus;
    mainProgram = "subsolve";
    platforms = lib.platforms.linux;
  };
}
