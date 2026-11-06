# Publishing Voltkit

Voltkit ships as native packages:

| Where | For | Built by |
|---|---|---|
| apt repository on GitHub Pages | Ubuntu, Debian, Mint, Pop!_OS, Zorin | the release workflow, automatically |
| Fedora COPR | Fedora | COPR's build servers, from `packaging/rpm/voltkit.spec` |

Both install a login entry for the background service, and Voltkit adds its shortcuts on first
launch, so users only install and open it.

## One-time setup

### Fedora COPR

1. Create a Fedora account at <https://accounts.fedoraproject.org>.
2. Log in to <https://copr.fedorainfracloud.org> and choose **New Project**:
   - **Project name:** `voltkit`
   - **Chroots:** the current Fedora releases for `x86_64` (and `aarch64` if you like)
   - **Enable internet access during builds:** on (Cargo downloads the Rust crates)
   - **Instructions:** `sudo dnf copr enable arios/voltkit && sudo dnf install voltkit`
3. In the project, open **Packages → New Package → SCM**:
   - **Package name:** `voltkit`
   - **Clone url:** `https://github.com/arios4k/voltkit`
   - **Committish:** leave empty (the spec on `main` downloads the release it names)
   - **Spec file:** `packaging/rpm/voltkit.spec`
   - **How to build SRPM:** `make_srpm`
4. Save, then **Rebuild** the package.
5. To build automatically on each release: in the project open **Settings → Integrations**, copy the
   **Custom webhook** URL for the `voltkit` package, and save it as the repository secret
   `COPR_WEBHOOK_URL` (*Settings → Secrets and variables → Actions*). The URL contains a secret, so
   keep it private.

## Every release

1. Set the new version in `Cargo.toml` and `packaging/rpm/voltkit.spec` (`Version`, `Release: 1`,
   and a `%changelog` entry). `cargo test` fails until they match. Regenerate the man pages
   (`cargo run -- manpages man`).
2. Commit, then tag and push: `git tag vX.Y.Z && git push origin main vX.Y.Z`. The release workflow
   publishes the GitHub Release and the apt repository.
3. COPR builds by itself: the last job of the release workflow calls the COPR webhook once the tag is
   pushed. Without the `COPR_WEBHOOK_URL` secret, press **Rebuild** on the `voltkit` package (or run
   `copr-cli build-package voltkit --name voltkit`).

### If the release workflow fails at "Deploy to GitHub Pages"

GitHub's Pages deployment can time out (`Current status: updating_pages`, then `Timeout reached`).
The GitHub Release and its `.deb` are already published by then, but the apt index isn't updated and
the COPR job is skipped, because it waits for the release job. A Pages deployment is tied to the
commit, so re-running the same tag (or running the workflow again on the same commit) fails at once
with `Deployment cancelled`. To recover:

1. Put any small commit on `main` (a docs fix is enough), so Pages has a fresh commit to deploy.
2. Run the workflow by hand (*Actions → Release → Run workflow* on `main`, or
   `gh workflow run release.yml --ref main`). It rebuilds the apt repository from the existing
   releases and doesn't touch them.
3. Ask COPR for the build yourself, since the tag run skipped it: press **Rebuild** on the `voltkit`
   package, or run `copr-cli build-package voltkit --name voltkit`.

CI builds the Fedora package in a container on every push, so problems show up before a
release.
