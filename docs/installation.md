# Installation and linking

Use tblite **0.7.x**, built with its C API enabled. These instructions pin 0.7.0
and GNU Fortran. Meson is upstream's recommended build route for the complete
library: [upstream installation guide](https://tblite.readthedocs.io/en/latest/installation.html).

`tools/build-native.py` is an explicit installation tool, independent of Cargo.
It verifies the release archive's SHA-256, checks out the dependency commits
resolved from the release's wraps, builds with Meson, and installs into the
prefix you provide. Prerequisites and compiler versions remain platform package
manager inputs; this is reproducible source selection, not a promise of identical
binaries across different toolchains. Build in a path without spaces for the
most reliable Fortran/pkg-config experience.

The installer applies a small visibility fix to the pinned source: 13 released
C entry points are explicitly declared `PUBLIC` in their Fortran modules. This
avoids missing shared-library exports on macOS with affected GNU Fortran
versions (see [GCC PR126872](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=126872)).
It changes no C signatures or procedure bodies. Pristine and already-patched
files are accepted; other edits to these five source files produce an error
instead of being overwritten. The installed native library remains LGPL-licensed.

## Linux (x86-64)

For Debian/Ubuntu, install GNU Fortran, C/C++, OpenBLAS, HDF5 including Fortran
modules, Python 3.10+, Meson, Ninja, Git and pkg-config:

```sh
sudo apt-get update
sudo apt-get install gfortran gcc g++ libopenblas-dev libhdf5-dev \
    python3 meson ninja-build git pkg-config
export FC=gfortran CC=gcc CXX=g++
python3 tools/build-native.py --prefix "$PWD/.native/install" --library both
export TBLITE_DIR="$PWD/.native/install"
export LD_LIBRARY_PATH="$TBLITE_DIR/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
cargo run -p tblite --example singlepoint
```

Avoid Meson 1.8.0, which upstream excludes. To choose a newer Meson without
changing system Python, install it in a Python virtual environment.

## macOS (Apple Silicon and Intel)

Use Homebrew packages for the **same architecture as your Rust target**:

```sh
brew install gcc openblas hdf5 libaec meson ninja pkgconf python git
export FC="$(brew --prefix gcc)/bin/gfortran"
export CC=clang CXX=clang++
export MACOSX_DEPLOYMENT_TARGET="$(sw_vers -productVersion | cut -d. -f1,2)"
export PKG_CONFIG_PATH="$(brew --prefix openblas)/lib/pkgconfig:$(brew --prefix hdf5)/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LIBRARY_PATH="$(brew --prefix libaec)/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
export PATH="$(brew --prefix hdf5)/bin:$PATH"
python3 tools/build-native.py --prefix "$PWD/.native/install" --library both
export TBLITE_DIR="$PWD/.native/install"
export DYLD_LIBRARY_PATH="$TBLITE_DIR/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
cargo run -p tblite --example singlepoint
```

Homebrew's HDF5 must include Fortran support and match the selected GNU compiler.
`LIBRARY_PATH` supplies its compression libraries (`sz` and `aec`) when linking
statically; HDF5's pkg-config metadata can omit their Homebrew search directory.
The deployment target aligns Rust and native objects with the current macOS
version. Targeting an older OS also requires dependencies built for that OS.
The CI matrix contains separate `macos-15` ARM64 and `macos-15-intel` x86-64 jobs.
Both architectures passed CI during 0.1.0 preparation; see the linked run in
[development.md](development.md). No macOS machine was available for local
validation, and each release still requires CI for its own commit.

## Windows x86-64, MSVC Rust

Install the MSVC C++ Build Tools and Windows SDK, and use the
`x86_64-pc-windows-msvc` Rust target. Install [MSYS2](https://www.msys2.org/) and
open its **UCRT64** shell. Update MSYS2 as directed by its installer, then:

```sh
pacman -S --needed git mingw-w64-ucrt-x86_64-gcc-fortran \
    mingw-w64-ucrt-x86_64-meson mingw-w64-ucrt-x86_64-ninja \
    mingw-w64-ucrt-x86_64-openblas mingw-w64-ucrt-x86_64-hdf5
cd /c/path/to/tblite-rs
export FC=gfortran CC=gcc CXX=g++
python tools/build-native.py --prefix "$PWD/.native/install" --library shared
```

The installer handles the ddX symlink representation on Windows and applies
upstream's `-Wl,--allow-multiple-definition` linker workaround. Use one consistent
UCRT64 dependency stack. The separate GNU DLL owns all Fortran allocations;
MSVC Rust exchanges only the C ABI's scalar values, buffers and opaque handles.

In PowerShell, from the repository root:

```powershell
./tools/windows-import-library.ps1 -Prefix .native/install
$env:TBLITE_DIR = (Resolve-Path .native/install).Path
$env:PATH = "$env:TBLITE_DIR\bin;C:\msys64\ucrt64\bin;$env:PATH"
$env:OMP_NUM_THREADS = '1'
$env:OPENBLAS_NUM_THREADS = '1'
cargo run -p tblite --example singlepoint
```

Adjust the MSYS2 path to your installation. The PowerShell tool verifies all
117 public exports and generates `lib/tblite.lib` from the actual DLL using
MSVC's `dumpbin.exe` and `lib.exe`. A GNU `.dll.a` is not used as the MSVC import
library. The import library is a build-time input; the matching tblite DLL and
its dependency DLLs are needed at runtime. Do not substitute Fortran modules or
runtime DLLs from an unrelated GCC installation.

If Windows shows a system-error dialog naming `libgcc_s_seh-1.dll`,
`libgfortran-5.dll` or another runtime DLL, the executable could not start.
Check that the same UCRT64 `bin` directory used for the native build is in the
launched process's `PATH`. Setting `TBLITE_DIR` alone only helps Cargo find the
library at build time. The DLL may already be installed but outside that search
path; reinstalling tblite is not needed in that case. For the benchmark runner,
pass `--runtime-dir C:/msys64/ucrt64/bin` with your actual installation path.
The runner logs these startup errors and stops without Windows loader dialogs.

## Cargo discovery and runtime discovery

Set **one** of:

- `TBLITE_DIR`: an absolute installation prefix containing `include/tblite.h`,
  `lib/` (or `lib64/`), and Meson's `lib/pkgconfig/tblite.pc`.
- `PKG_CONFIG_PATH`: a search path containing the directory with `tblite.pc`.
  Cargo checks the version and uses pkg-config's library metadata.

`TBLITE_DIR` takes precedence. Dynamic linking through that prefix needs no
pkg-config executable. Static linking always uses pkg-config to discover the
transitive libraries. Missing installations, unsupported versions, absent
archives and missing MSVC import libraries produce build errors with next steps.

Build-time discovery does not configure your application's runtime loader.
During development, set `PATH` on Windows, `LD_LIBRARY_PATH` on Linux, or
`DYLD_LIBRARY_PATH` on macOS as above. For installed applications, configure an
appropriate rpath/install name or package the libraries in the application's
library directory. macOS can strip `DYLD_*` variables from protected processes;
an application rpath is more suitable for distribution. For example, a macOS
development rpath can be supplied with
`RUSTFLAGS="-C link-arg=-Wl,-rpath,$TBLITE_DIR/lib"`.

The binding does not silently embed this checkout's absolute path in your
application. Inspect dependencies with `ldd`, `otool -L`, or `dumpbin /dependents`.

## Static tblite on Linux/macOS

Install with `--library both` or `--library static`, then:

```sh
cargo run -p tblite --features static --example singlepoint
cargo test --workspace --features tblite/static,tblite-sys/abi-tests
```

The feature requires `libtblite.a` and shared external dependencies. It bundles
tblite (including the installer's internal Fortran subprojects), while BLAS,
GNU Fortran/OpenMP and optional HDF5 remain shared. This avoids accidentally
embedding a second compiler/BLAS runtime from Homebrew's static archives.
Fully static executables are not supported. The build script reads the complete
dependency list from `pkg-config --static` and corrects two metadata omissions:
GNU `-fopenmp` requires `gomp`, and HDF5 requires its Fortran library.
Static linking is initially exercised with the GNU toolchain in these recipes.

If GNU runtime libraries are outside the platform linker search paths, the
build script asks `FC` (default `gfortran`) for their location. Alternatively set
`TBLITE_FORTRAN_LIB_DIR` to the directory containing them. This explicit setting
is also available for cross builds, which are not part of the tested matrix.
An existing static installation must supply its dependency libraries and
pkg-config files; an archive alone is insufficient. Windows static linking
returns an intentional build error.

## Optional capabilities

The default installer enables ddX and HDF5 and disables TREXIO. Pass
`--trexio enabled` after installing a compatible TREXIO library to enable its
wavefunction output. TREXIO output is exposed but was not exercised locally.
Use `--minimal` to disable ddX and HDF5; other features remain available.

```sh
python3 tools/build-native.py --prefix "$PWD/.native/minimal" \
    --build-dir "$PWD/.native/build-minimal" --minimal
```

There is no 0.7.0 C feature-query function. Optional operations are always in the
Rust API and return the native error if unavailable. Set
`TBLITE_TEST_OPTIONAL=full` for tests requiring ddX and HDF5, or `minimal` to
require that both are absent. With neither set, tests accept either installation
and check that unavailable features report an error.
