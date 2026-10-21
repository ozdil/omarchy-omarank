# Maintainer: Ozan Özdil <ozdil>
pkgname=omarchy-omarank
pkgver=1.4.0
pkgrel=1
pkgdesc="Hardware benchmark, humorous tier ranking, and OmaStat survey widget for Omarchy"
arch=('x86_64')
url="https://github.com/ozdil/omarchy-omarank"
license=('MIT')
depends=('glibc' 'gcc-libs' 'curl')
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
    install -Dm755 "target/release/omarank-engine" "${pkgdir}/usr/bin/omarank-engine"
    install -Dm755 "omarank-dashboard" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-dashboard"
    install -Dm755 "omarank-status" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-status"
    install -Dm755 "target/release/omarank-engine" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-engine"
    install -Dm644 "manifest.json" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/manifest.json"
    install -Dm644 "Panel.qml" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/Panel.qml"
    install -Dm644 "qml/shell.qml" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/qml/shell.qml"
    install -Dm644 "qml/MainWindow.qml" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/qml/MainWindow.qml"
    install -Dm644 "qml/theme/Theme.qml" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/qml/theme/Theme.qml"
    install -Dm644 "qml/theme/qmldir" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/qml/theme/qmldir"
    install -Dm644 "README.md" "${pkgdir}/usr/share/doc/${pkgname}/README.md"
    install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
