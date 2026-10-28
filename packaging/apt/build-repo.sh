#!/usr/bin/env bash
# Builds the signed Voltkit APT repository that GitHub Pages serves.
#
#   APT_KEY_ID=<gpg key id> packaging/apt/build-repo.sh <dir with .debs> <output dir>
#
# Layout (one suite, "stable", component "main"):
#   pool/main/v/voltkit/*.deb
#   dists/stable/{Release,Release.gpg,InRelease}
#   dists/stable/main/binary-amd64/Packages{,.gz}
#   voltkit-archive-keyring.gpg   public key users install
set -euo pipefail

debs=${1:?usage: build-repo.sh <deb dir> <output dir>}
out=${2:?usage: build-repo.sh <deb dir> <output dir>}
key=${APT_KEY_ID:?set APT_KEY_ID to the signing key}
here=$(cd "$(dirname "$0")" && pwd)

rm -rf "$out"
mkdir -p "$out/pool/main/v/voltkit" "$out/dists/stable/main/binary-amd64"
cp "$debs"/*.deb "$out/pool/main/v/voltkit/"
cd "$out"

apt-ftparchive packages pool > dists/stable/main/binary-amd64/Packages
gzip -9kn dists/stable/main/binary-amd64/Packages

apt-ftparchive \
    -o APT::FTPArchive::Release::Origin=Voltkit \
    -o APT::FTPArchive::Release::Label=Voltkit \
    -o APT::FTPArchive::Release::Suite=stable \
    -o APT::FTPArchive::Release::Codename=stable \
    -o APT::FTPArchive::Release::Architectures=amd64 \
    -o APT::FTPArchive::Release::Components=main \
    -o "APT::FTPArchive::Release::Description=Voltkit micro-utilities for Linux" \
    release dists/stable > dists/stable/Release

gpg --batch --yes --local-user "$key" --armor --detach-sign -o dists/stable/Release.gpg dists/stable/Release
gpg --batch --yes --local-user "$key" --clearsign -o dists/stable/InRelease dists/stable/Release
gpg --batch --yes --export "$key" > voltkit-archive-keyring.gpg

cp "$here/index.html" index.html
touch .nojekyll   # serve files as-is; Jekyll would skip some of them
echo "APT repository written to $out"
