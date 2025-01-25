let
	pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.11") {};
in
pkgs.mkShell {
	packages = with pkgs; [
		cargo
		rustc
		rustfmt

		openssl
		pkg-config

		ollama
	];

	shellHook = ''
		export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
	'';
}
