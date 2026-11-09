# Publishing a release

Release `tblite-sys` and `tblite` together at the workspace version. Then
create the GitHub release from the same commit, with the three runtime bundles
that CI built from it. The commands below use PowerShell. Run them one step at
a time and stop if any command fails. Set the version once:

```powershell
$version = '0.2.0'
$tag = "v$version"
```

## Accounts and tools

- Use current stable Rust/Cargo for release tooling (`rustup update stable`).
  The library's minimum supported Rust version remains 1.85.
- Sign in to [crates.io](https://crates.io), verify your account email, and
  create an [API token](https://crates.io/settings/tokens) that can publish
  both crates. Run `cargo login --registry crates-io` and paste the token
  at its prompt. Keep the token out of commands, source files and chat.
- Run `gh auth login` if needed, then `gh auth status` to check GitHub access
  to `choutkaj/tblite-rs`.

## Prepare the release

On a branch, update:

- the workspace version and the `tblite-sys` dependency version in `Cargo.toml`,
  then `Cargo.lock` with `cargo update --workspace`;
- the versions in `README.md`, `tblite/README.md` and `tblite-sys/README.md`;
- `CHANGELOG.md`: rename `Unreleased` to the version;
- the release notes, `docs/releases/$tag.md`.

Merge the branch into `main`, then update the local `main` checkout. A tag must
identify the commit actually published.

## Require CI for that commit

```powershell
git switch main
git pull --ff-only
if (git status --porcelain) { throw 'The release checkout must be clean.' }
$releaseCommit = git rev-parse HEAD
$releaseRun = gh run list --repo choutkaj/tblite-rs --commit $releaseCommit --workflow ci.yml --event push --limit 1 --json databaseId --jq '.[0].databaseId'
if (-not $releaseRun) { throw 'No CI run yet; retry after GitHub starts it.' }
gh run watch $releaseRun --repo choutkaj/tblite-rs --exit-status
if ($LASTEXITCODE -ne 0) { throw 'Release CI did not pass.' }
```

All eight jobs must pass. Linux/full also runs
`cargo publish --locked --workspace --dry-run`, which verifies both packages
without uploading. No Cargo publishing token is needed for that CI step.

## Collect the runtime bundles

The three bundle jobs keep `tblite-<tblite version>-<platform>.tar.gz` as run
artifacts. Download them and check that each was built from the release
commit:

```powershell
$bundles = Join-Path $env:TEMP "tblite-rs-$version-bundles"
if (Test-Path $bundles) { throw "$bundles exists; remove it first." }
gh run download $releaseRun --repo choutkaj/tblite-rs --pattern 'bundle-*' --dir $bundles
$archives = @(Get-ChildItem $bundles -Recurse -Filter 'tblite-*.tar.gz')
if ($archives.Count -ne 3) { throw 'Expected three bundle archives.' }
foreach ($archive in $archives) {
    $platform = $archive.Name -replace '^tblite-[0-9.]+-(.+)\.tar\.gz$', '$1'
    $sources = (tar -xOzf $archive.FullName "$platform/SOURCES.md") -join "`n"
    if ($sources -notmatch "at ``$releaseCommit``\.") { throw "$($archive.Name) is not from $releaseCommit." }
    tar -xzf $archive.FullName -C $bundles
}
$archives | Select-Object Name, Length
```

## Verify and publish to crates.io

This machine has an unrelated Cargo patch in the parent `repos/.cargo`
configuration. Run Cargo from the temporary directory with an explicit manifest
path to avoid inheriting that parent configuration. Keep this PowerShell session
open so the paths and variables remain set. Packaging and publishing compile
the packaged code, so first point them at the Windows bundle:

```powershell
$releaseManifest = (Resolve-Path Cargo.toml).Path
$env:TBLITE_DIR = Join-Path $bundles 'windows-x86_64'
$env:PATH = "$env:TBLITE_DIR\lib;$env:PATH"
$env:OMP_NUM_THREADS = '1'
$env:OPENBLAS_NUM_THREADS = '1'
```

On Linux or macOS, set `TBLITE_DIR` to that platform's bundle and the loader
path as described in [distribution.md](distribution.md); the Cargo and Git
commands are otherwise the same.

First verify both crates together. Cargo stages workspace dependencies during
the dry-run, so this works before the new `tblite-sys` exists on crates.io.
Use registry access: the offline workspace package check fails with Cargo's
internal `no hash listed for tblite-sys` error.

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
crates.io before retrying. Once the new `tblite-sys` exists, resume with the
`tblite` dry-run and publish commands; do not upload the raw crate again. If
code needs to change after a successful upload, prepare a new version for that
crate. Do not use `--no-verify` or `--allow-dirty` for the actual release.

## Tag and publish on GitHub

After both crates are published, tag the verified commit and create the release
with the bundles attached. The checked-in notes are the release body.

```powershell
if ((git rev-parse HEAD) -ne $releaseCommit) { throw 'HEAD changed since verification.' }
if (git status --porcelain) { throw 'The release checkout must be clean.' }
git tag -a $tag $releaseCommit -m "tblite-rs $version"
if ($LASTEXITCODE -ne 0) { throw 'Tag creation failed.' }
git push origin "refs/tags/$tag"
if ($LASTEXITCODE -ne 0) { throw 'Tag push failed.' }
gh release create $tag --repo choutkaj/tblite-rs --verify-tag --title "tblite-rs $version" --notes-file "docs/releases/$tag.md" $archives.FullName
```

GitHub supplies source archives for the tag. `--verify-tag` requires the pushed
tag instead of creating one from whatever happens to be the latest `main`
commit.

Check the GitHub release and its three assets, then `tblite-sys` and `tblite`
on crates.io. Allow docs.rs time to build, then check both crates' API docs.

References: [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html),
[`cargo publish`](https://doc.rust-lang.org/cargo/commands/cargo-publish.html),
[`gh release create`](https://cli.github.com/manual/gh_release_create),
[`gh run download`](https://cli.github.com/manual/gh_run_download).
