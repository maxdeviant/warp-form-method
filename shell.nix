with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "warp_form_method";

  buildInputs = [
    stdenv
    pkg-config
  ];
}
