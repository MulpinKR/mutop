# Maintainer: Your Name
pkgname=amber-tasks
pkgver=0.1.0
pkgrel=1
pkgdesc="A beautiful orange-themed TUI task manager"
arch=('x86_64' 'aarch64')
url="https://github.com/yourusername/amber-tasks"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=("task-manager-$pkgver.tar.gz::https://github.com/yourusername/amber-tasks/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "amber-tasks-$pkgver"
  cargo build --release --locked
}

package() {
  cd "amber-tasks-$pkgver"
  install -Dm755 "target/release/amber-tasks" "$pkgdir/usr/bin/amber-tasks"
}
