# Contributing

## Checks

Point `TBLITE_DIR` at a tblite 0.8 installation and its loader path at `lib/`
(see the README), then:

```sh
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --features tblite-sys/abi-tests -- -D warnings
cargo test --workspace --features tblite-sys/abi-tests
cargo test --workspace --features tblite/static,tblite-sys/abi-tests  # Linux, macOS
python3 tools/generate_bindings.py --check
python3 tools/check_coverage.py
python3 tools/check_memory.py  # Linux, needs Valgrind
```

`abi-tests` compiles a C probe against the installed headers, compares C and
Rust layouts, and resolves all 123 functions. `TBLITE_TEST_OPTIONAL=full` or
`minimal` makes the tests require ddX and HDF5 to be present or absent.
`check_memory.py` suppresses only tblite's known logger leak.

## Bindings

`tools/generate_bindings.py` writes `tblite-sys/src/bindings.rs`, `api.json`
and `tests/abi.c` from the headers in `tblite-sys/vendor/`. It accepts only
the declaration forms those headers use. `tools/check_coverage.py` lists the
Rust equivalent of every C function; add new functions there.

To move to a new tblite release, update the pins in `tools/build-native.py`
and `tools/bundle-native.py`, the vendored headers and the version checks.
Read the Fortran `bind(C)` bodies, not only the headers: some header
annotations are wrong (see the `tblite-sys` crate docs).

## Benchmark

`benchmarks/mb16-43/README.md` describes the run. To renew a platform's
report, run into a new folder, replace `benchmarks/mb16-43/results/<platform>`
with it, and redraw the figure:

```sh
python3 -m venv .native/parity-venv
.native/parity-venv/bin/python -m pip install -r tools/parity-requirements.txt
cargo build --locked -p tblite --example parity_export
.native/parity-venv/bin/python tools/benchmark_parity.py run \
  --native .native/bundle/install/bin/tblite \
  --library "$TBLITE_DIR/lib/libtblite.so.0" \
  --rust target/debug/examples/parity_export \
  --output .native/parity-linux
.native/parity-venv/bin/python tools/benchmark_parity.py platforms
```

The native `tblite` program comes from the bundle's build directory. On
Windows, pass `--runtime-dir` for the DLLs' directory.

## Bundle pins

- Linux, macOS: solve again from the specs in the header of
  `tools/bundle/conda-*.txt` and replace its package lines with the new
  environment's `URL#sha256:HASH` lines. For macOS, set `CONDA_OVERRIDE_OSX=11.0`
  and pass `--platform osx-arm64`.
- Windows: install the new packages in MSYS2 and replace their lines in
  `tools/bundle/msys2-ucrt64.txt` with the version and the package file's SHA-256.

Then build all three bundles, run the tests against each, and review the
changes to `SOURCES.md`.

## Releasing

1. On a branch, set the version in `Cargo.toml` (the workspace and the
   `tblite-sys` dependency), run `cargo update --workspace`, update the
   version in the three READMEs and add its `CHANGELOG.md` section. Merge.
2. In PowerShell, on `main` at the merge commit:

```powershell
$version = '0.2.0'; $tag = "v$version"
git switch main; git pull --ff-only
$commit = git rev-parse HEAD
$run = gh run list --commit $commit --workflow ci.yml --event push --limit 1 --json databaseId --jq '.[0].databaseId'
gh run watch $run --exit-status  # all eight jobs must pass

# The bundles CI built from this commit.
$dist = Join-Path $env:TEMP "tblite-rs-$version"
gh run download $run --pattern 'bundle-*' --dir $dist
$archives = @(Get-ChildItem $dist -Recurse -Filter 'tblite-*.tar.gz')
foreach ($a in $archives) {
    $platform = $a.Name -replace '^tblite-[0-9.]+-(.+)\.tar\.gz$', '$1'
    $sources = (tar -xOzf $a.FullName "$platform/SOURCES.md") -join "`n"
    if ($sources -notmatch "at ``$commit``\.") { throw "$($a.Name) is not from $commit" }
}
# Windows tar cannot create the Linux and macOS library links; only this one is needed.
tar -xzf ($archives | Where-Object Name -like '*windows*').FullName -C $dist

# crates.io, built against the Windows bundle. From $env:TEMP, no parent
# .cargo/config.toml applies.
$manifest = (Resolve-Path Cargo.toml).Path
$env:TBLITE_DIR = "$dist\windows-x86_64"; $env:PATH = "$env:TBLITE_DIR\lib;$env:PATH"
Push-Location $env:TEMP
cargo publish --manifest-path $manifest --registry crates-io --locked --workspace --dry-run
cargo publish --manifest-path $manifest --registry crates-io --locked -p tblite-sys
cargo publish --manifest-path $manifest --registry crates-io --locked -p tblite
Pop-Location

# Tag and GitHub release; the notes are the CHANGELOG section.
$section = (Get-Content CHANGELOG.md -Raw) -split '(?m)^## ' | Where-Object { $_ -match "^$([regex]::Escape($version))\r?\n" }
[IO.File]::WriteAllText("$dist\notes.md", ($section -replace '^.*\r?\n', '').Trim())
git tag -a $tag $commit -m "tblite-rs $version"; git push origin "refs/tags/$tag"
gh release create $tag --verify-tag --title "tblite-rs $version" --notes-file "$dist\notes.md" $archives.FullName
```

Stop at the first failing command. If `tblite-sys` is published but `tblite`
fails, fix the cause and publish `tblite` alone; a published version cannot
be uploaded again.
