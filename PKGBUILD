# Maintainer: Ozan Özdil <ozdil>
pkgname=omarchy-omarank
pkgver=1.5.0
pkgrel=1
pkgdesc="Hardware benchmark, humorous tier ranking, and OmaStat survey widget for Omarchy"
arch=('x86_64')
url="https://github.com/ozdil/omarchy-omarank"
license=('MIT')
# Qt 6.12 LTS is the tested baseline (QtQuick's new `Color` type is what the qualified
# `Commons.Color` palette reads in Panel.qml guard against). Wayland-only: no X11 deps.
depends=('glibc' 'gcc-libs' 'curl' 'quickshell' 'qt6-declarative>=6.12' 'qt6-wayland' 'wl-clipboard' 'xdg-utils')
optdepends=('hyprland: monitor detection through hyprctl (falls back to DRM sysfs)'
            'pciutils: GPU and chipset names (lspci)'
            'inxi: richer hardware details'
            'xdg-terminal-exec: terminal dashboard outside Omarchy')
install=omarchy-omarank.install
makedepends=('cargo' 'rust' 'git')
source=("git+https://github.com/ozdil/omarchy-omarank.git#tag=v${pkgver}")
sha256sums=('SKIP')

prepare() {
    cd "${srcdir}/omarchy-omarank"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "${srcdir}/omarchy-omarank"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --release --frozen
}

check() {
    cd "${srcdir}/omarchy-omarank"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen
}

package() {
    cd "${srcdir}/omarchy-omarank"
    # Omarchy only discovers bar widgets under ~/.config/omarchy/plugins/<id>/ (see
    # omarchy-omarank.install); the package ships the engine system-wide plus a read-only
    # copy of the plugin files that the standalone dashboard runs from.
    local share="${pkgdir}/usr/share/omarchy-omarank"
    install -Dm755 "target/release/omarank-engine" "${pkgdir}/usr/bin/omarank-engine"
    install -Dm755 "target/release/omarank-engine" "${share}/omarank-engine"
    install -Dm755 "omarank-dashboard" "${share}/omarank-dashboard"
    install -Dm755 "omarank-status" "${share}/omarank-status"
    install -Dm644 "manifest.json" "${share}/manifest.json"
    install -Dm644 "Panel.qml" "${share}/Panel.qml"
    install -Dm644 "qml/shell.qml" "${share}/qml/shell.qml"
    install -Dm644 "qml/MainWindow.qml" "${share}/qml/MainWindow.qml"
    install -Dm644 "qml/theme/Theme.qml" "${share}/qml/theme/Theme.qml"
    install -Dm644 "qml/theme/qmldir" "${share}/qml/theme/qmldir"
    install -dm755 "${pkgdir}/usr/bin"
    ln -s /usr/share/omarchy-omarank/omarank-dashboard "${pkgdir}/usr/bin/omarank-dashboard"
    install -Dm644 "omarank.desktop" "${pkgdir}/usr/share/applications/omarank.desktop"
    install -Dm644 "README.md" "${pkgdir}/usr/share/doc/${pkgname}/README.md"
    install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
