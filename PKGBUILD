# Maintainer: Ozan Özdil <ozdil>
pkgname=omarchy-omarank
pkgver=1.2.0
pkgrel=1
pkgdesc="Hardware benchmark, humorous tier ranking, and OmaStat survey widget for Omarchy"
arch=('x86_64')
url="https://github.com/ozdil/omarchy-omarank"
license=('MIT')
depends=('glibc' 'gcc-libs' 'curl')
makedepends=('cargo' 'rust')

build() {
    cd "${startdir}"
    cargo build --release --locked
}

package() {
    cd "${startdir}"
    install -Dm755 "target/release/omarank-engine" "${pkgdir}/usr/bin/omarank-engine"
    install -Dm755 "omarank-dashboard" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-dashboard"
    install -Dm755 "omarank-status" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-status"
    install -Dm755 "target/release/omarank-engine" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/omarank-engine"
    install -Dm644 "manifest.json" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/manifest.json"
    install -Dm644 "Panel.qml" "${pkgdir}/usr/share/omarchy/plugins/ozdil.omarank/Panel.qml"
    install -Dm644 "README.md" "${pkgdir}/usr/share/doc/${pkgname}/README.md"
    install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
