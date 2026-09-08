# Publishing 0.1.0

Release `tblite-sys` and `tblite` together at version **0.1.0**, then create the
GitHub release from the same commit. The commands below use PowerShell and are
intended to be run one step at a time. Stop if any command fails.

## Accounts and tools

- Use current stable Rust/Cargo for release tooling (`rustup update stable`).
  The library's minimum supported Rust version remains 1.85.
- Sign in to [crates.io](https://crates.io), verify your account email, and
  create an [API token](https://crates.io/settings/tokens) that can publish
  both new crates. Run `cargo login --registry crates-io` and paste the token
  at its prompt. Keep the token out of commands, source files and chat.
- Run `gh auth login` if needed, then `gh auth status` to check GitHub access
  to `choutkaj/tblite-rs`.
- Keep the native tblite 0.7.x installation available while packaging and
  publishing. Both commands compile the packaged code.

## Commit the release preparation

Start in the repository on `main`, with only the intended release changes.
Review the diff, then commit the prepared files:

```powershell
git diff --check
git status --short
git diff
git add README.md CHANGELOG.md tblite/README.md tblite-sys/README.md docs/development.md docs/installation.md docs/releasing.md docs/releases/v0.1.0.md .github/workflows/ci.yml
git commit -m "Prepare 0.1.0 release"
git push origin main
```

If using a pull request, merge the preparation and update the local `main`
checkout before continuing. A tag must identify the commit actually published.

## Require CI for that commit

```powershell
$releaseCommit = git rev-parse HEAD
$releaseRun = gh run list --commit $releaseCommit --workflow ci.yml --limit 1 --json databaseId --jq '.[0].databaseId'
if (-not $releaseRun) { throw 'No CI run yet; retry after GitHub starts it.' }
gh run watch $releaseRun --exit-status
if ($LASTEXITCODE -ne 0) { throw 'Release CI did not pass.' }
if (git status --porcelain) { throw 'The release checkout must be clean.' }
```

Require all five platform jobs to pass. Linux/full also runs
`cargo publish --locked --workspace --dry-run`, which verifies both packages
without uploading. No Cargo publishing token is needed for that CI step.

## Configure the native library

For the existing verified Windows installation in this checkout:

```powershell
$releaseManifest = (Resolve-Path Cargo.toml).Path
$env:TBLITE_DIR = (Resolve-Path .native/install-win).Path
$releaseRuntime = (Resolve-Path .native/msys64/ucrt64/bin).Path
$env:PATH = "$env:TBLITE_DIR\bin;$releaseRuntime;$env:PATH"
$env:OMP_NUM_THREADS = '1'
$env:OPENBLAS_NUM_THREADS = '1'
```

For another checkout or machine, use the paths from
[installation.md](installation.md). On Linux/macOS, set `TBLITE_DIR` and the
appropriate loader path as described there; the Cargo and Git commands are
otherwise the same.

## Verify and publish to crates.io

This machine has an unrelated Cargo patch in the parent `repos/.cargo`
configuration. Run Cargo from the temporary directory with an explicit manifest
path to avoid inheriting that parent configuration. Keep this PowerShell session
open so the native paths and `$releaseManifest` remain set.

First verify both crates together. This also works before `tblite-sys` exists
on crates.io because Cargo stages workspace dependencies during the dry-run.
Use registry access: the offline workspace package check encountered Cargo's
internal `no hash listed for tblite-sys` error during release preparation.

```powershell
Push-Location $env:TEMP
try {
    cargo +stable publish --manifest-path $releaseManifest --registry crates-io --locked --workspace --dry-run
    if ($LASTEXITCODE -ne 0) { throw 'Package verification failed.' }
} finally {
    Pop-Location
}
```

Then publish the raw crate first, wait for Cargo to confirm it is available in
the registry, verify the safe crate against it, and publish the safe crate:

```powershell
Push-Location $env:TEMP
try {
    cargo +stable publish --manifest-path $releaseManifest --registry crates-io --locked -p tblite-sys
    if ($LASTEXITCODE -ne 0) { throw 'Check tblite-sys publication before continuing.' }
    cargo +stable publish --manifest-path $releaseManifest --registry crates-io --locked -p tblite --dry-run
    if ($LASTEXITCODE -ne 0) { throw 'tblite verification failed.' }
    cargo +stable publish --manifest-path $releaseManifest --registry crates-io --locked -p tblite
    if ($LASTEXITCODE -ne 0) { throw 'Check tblite publication before continuing.' }
} finally {
    Pop-Location
}
```

If an upload succeeds but waiting for the index times out, check the version on
crates.io before retrying. Once `tblite-sys` 0.1.0 exists, resume with the `tblite`
dry-run and publish commands; do not upload the raw crate again. If code needs
to change after a successful upload, prepare a new version for that crate.
Do not use `--no-verify` or `--allow-dirty` for the actual release.

## Tag and publish on GitHub

After both crates are published, return to the repository and tag the verified
commit. The checked-in notes are ready to use as the GitHub release body.

```powershell
if ((git rev-parse HEAD) -ne $releaseCommit) { throw 'HEAD changed since verification.' }
if (git status --porcelain) { throw 'The release checkout must be clean.' }
git tag -a v0.1.0 $releaseCommit -m "tblite-rs 0.1.0"
if ($LASTEXITCODE -ne 0) { throw 'Tag creation failed.' }
git push origin refs/tags/v0.1.0
if ($LASTEXITCODE -ne 0) { throw 'Tag push failed.' }
gh release create v0.1.0 --repo choutkaj/tblite-rs --verify-tag --title "tblite-rs 0.1.0" --notes-file docs/releases/v0.1.0.md
```

GitHub supplies source archives for the tag. This release does not ship native
binaries. `--verify-tag` requires the pushed tag instead of creating one from
whatever happens to be the latest `main` commit.

Check the [GitHub release](https://github.com/choutkaj/tblite-rs/releases/tag/v0.1.0),
[`tblite-sys` 0.1.0](https://crates.io/crates/tblite-sys/0.1.0), and
[`tblite` 0.1.0](https://crates.io/crates/tblite/0.1.0).
Allow docs.rs time to build, then check the
[`tblite` API docs](https://docs.rs/tblite/0.1.0/tblite/) and
[`tblite-sys` API docs](https://docs.rs/tblite-sys/0.1.0/tblite_sys/).

References: [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html),
[`cargo publish`](https://doc.rust-lang.org/cargo/commands/cargo-publish.html),
[`gh release create`](https://cli.github.com/manual/gh_release_create).
