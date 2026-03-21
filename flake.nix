{
  description = "Tauri development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        libs = with pkgs; [
          gtk3
          glib
          dbus
          openssl
          cairo
          gdk-pixbuf
          pango
          harfbuzz
          librsvg
          libsoup_3
          webkitgtk_4_1
          glib-networking
        ];
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            cargo-tauri

            # Typst LSP + `tinymist preview` (live preview in PaperDesk)
            tinymist

            # `xdg-open` for tooling; Tauri AppImage bundling also copies `/usr/bin/xdg-open` if present
            xdg-utils

            nodejs_20
            pkg-config
            gobject-introspection
            wrapGAppsHook4
          ] ++ libs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
          XDG_DATA_DIRS =
            "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:"
            + "${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:"
            + "$XDG_DATA_DIRS";

          GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules/";
          OPENSSL_NO_VENDOR = 1;

          shellHook = ''
            export npm_config_prefix="$PWD/.npm-global"
            export PATH="$npm_config_prefix/bin:$PATH"
            echo "Tauri dev shell aktiv"
          '';
        };
      });
}
