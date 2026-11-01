#!/usr/bin/env bash
# After tagging a release, puts the checksum of its source tarball into the
# AUR package files (the Fedora spec doesn't need one).
#
#   packaging/update-checksums.sh
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml" | head -1)
url="https://github.com/arios4k/voltkit/archive/refs/tags/v$version.tar.gz"
sum=$(curl -fsSL "$url" | sha256sum | cut -d' ' -f1)

sed -i "s/^sha256sums=.*/sha256sums=('$sum')/" "$repo/packaging/aur/PKGBUILD"
sed -i "s/^\tsha256sums = .*/\tsha256sums = $sum/" "$repo/packaging/aur/.SRCINFO"
echo "v$version: $sum"
