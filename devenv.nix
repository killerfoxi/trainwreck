{
  pkgs,
  lib,
  config,
  ...
}:
{
  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";
    # wasm32 target required by crates/web (built with Trunk)
    targets = [ "wasm32-unknown-unknown" ];
  };

  # https://devenv.sh/packages/
  packages = [
    pkgs.cargo-edit
    pkgs.cmake      # required by aws-lc-sys (transitive dep of reqwest/rustls)
    pkgs.protobuf   # provides protoc, required by prost-build / protox at build time
    pkgs.trunk      # WASM bundler for crates/web
  ];
}
