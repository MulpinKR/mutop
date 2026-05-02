# Maintainer: Mulpin <mulpin@aur.archlinux.org>
pkgname=mtop
pkgver=0.2.0
pkgrel=1
pkgdesc="A beautiful orange-themed TUI process manager (btop alternative in Rust)"
arch=('x86_64' 'aarch64')
url="https://github.com/MulpinKR/mtop"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust' 'cargo' 'git')
conflicts=('amber-tasks')
provides=('mtop')
source=("mtop-$pkgver.tar.gz::https://github.com/MulpinKR/mtop/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "mtop-$pkgver"
  cargo build --release --locked
}

package() {
  cd "mtop-$pkgver"
  install -Dm755 "target/release/mtop" "$pkgdir/usr/bin/mtop"
  install -Dm644 "mtop.desktop" "$pkgdir/usr/share/applications/mtop.desktop"
}
