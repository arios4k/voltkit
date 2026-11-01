# COPR "make srpm" build method: COPR runs `make -f .copr/Makefile srpm
# outdir=DIR spec=SPEC` in a Fedora chroot with network access. This downloads
# the release tarball named in the spec and builds the source RPM from it.
spec ?= packaging/rpm/voltkit.spec
outdir ?= .

srpm:
	dnf -y install rpm-build rpmdevtools
	spectool -g -C $(outdir) $(spec)
	rpmbuild -bs --define "_sourcedir $(outdir)" --define "_srcrpmdir $(outdir)" $(spec)
