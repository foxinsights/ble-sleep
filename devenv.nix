{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages =
    with pkgs;
    [
      espflash
      moreutils
    ];

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  languages.rust = {
    enable = true;
    channel = "nightly";
    version = "2026-04-21";
    components = [
      "rustc"
      "rust-src"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "miri"
    ];
    targets = [
      "riscv32imac-unknown-none-elf"
      "x86_64-unknown-linux-gnu"
    ];
  };

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
