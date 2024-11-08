{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    let
      pkgs = import inputs.nixpkgs {
        system = "x86_64-linux";
        overlays = [
          (import inputs.rust-overlay)
        ];
      };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
	LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
	  openssl.dev
	];

	OPENSSL_DEV = pkgs.openssl.dev;

        packages = with pkgs; [
	  openssl
	  pkg-config
          (rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          })
        ];
      };
    };
}
