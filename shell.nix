with import <nixpkgs> { };
mkShell {
  buildInputs = [
    rustup
    gcc
    bacon
    pkg-config
    openssl.dev
  ];
}
