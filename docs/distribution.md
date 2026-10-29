# Distributing tblite with an application

`tools/bundle-native.py` builds a redistributable **tblite 0.7.0 runtime
bundle**: the shared library, every non-system library it loads, their licence
texts and a `SOURCES.md` that records versions, checksums, build settings and
source locations. An application ships `lib/`, `licenses/` and `SOURCES.md`
next to its executable, so tblite stays a separate, replaceable shared library.
Never enable the `static` feature for a distributed application; see
[licensing](licensing.md).

| Platform | Toolchain | Runs on |
| --- | --- | --- |
| Windows x86-64 | MSYS2 UCRT64 GCC 16.2 and OpenBLAS, pinned in `tools/bundle/msys2-ucrt64.txt` | Windows 10 and later (Universal C Runtime) |
| Linux x86-64 | conda-forge GCC 16.2 and OpenBLAS against the glibc 2.28 sysroot, pinned in `tools/bundle/conda-linux-64.txt` | glibc 2.28 and later: RHEL, Rocky and Alma 8 and 9, Ubuntu 20.04 and later |

macOS is not covered yet. Both platforms build the same sources as
`tools/build-native.py`: the tblite 0.7.0 release with the dependency commits of
its Meson wraps and the C API visibility fix (see [installation](installation.md)),
with ddX and without HDF5 or TREXIO.

## Layout

```text
include/            tblite.h and tblite/*.h
lib/                Windows: libtblite-0.dll, libopenblas.dll, libgfortran-5.dll,
                      libquadmath-0.dll, libgomp-1.dll, libgcc_s_seh-1.dll,
                      libwinpthread-1.dll, and tblite.lib (MSVC import library)
                    Linux: libtblite.so.0 (and the libtblite.so link), libopenblas.so.0,
                      libgfortran.so.5, libquadmath.so.0, libgomp.so.1, libgcc_s.so.1
lib/pkgconfig/      tblite.pc, relocatable
licenses/           tblite, dftd4, s-dftd3, ddx, multicharge, mctc-lib, toml-f, jonquil,
                    gcc, openblas, and winpthreads on Windows
SOURCES.md          every file's SHA-256 and licence; every source and package
```

The bundle is also an installation prefix: set `TBLITE_DIR` to it to build
against it. `include/`, `tblite.lib`, `libtblite.so` and `pkgconfig/` are needed
only for building. Linux libraries keep their sonames and each has the RUNPATH
`$ORIGIN`, so they find each other in `lib/`.

## Building a bundle

Both platforms need Git and Python 3.10 or later. The output directory must be
new or empty; the work directory (default `.native/bundle`) keeps sources, the
build tree and, on Linux, the toolchain, so a second run is quick.

### Windows x86-64

Install [MSYS2](https://www.msys2.org/) and the MSVC C++ Build Tools. Install the
pinned UCRT64 packages in a UCRT64 shell; if any is missing or at another version,
the script stops and prints this command with every URL:

```sh
pacman -U --needed https://repo.msys2.org/mingw/ucrt64/mingw-w64-ucrt-x86_64-gcc-16.2.0-3-any.pkg.tar.zst ...
```

Then, from any shell:

```powershell
python tools/bundle-native.py --msys2 C:\msys64 --out .native/tblite-windows-x86_64
```

### Linux x86-64

Install [micromamba](https://mamba.readthedocs.io/en/latest/installation/micromamba-installation.html);
no root access, system compiler or system BLAS is needed:

```sh
python3 tools/bundle-native.py --out .native/tblite-linux-x86_64
```

The script creates the conda-forge toolchain from the lock (about 260 MB of
packages, each verified by SHA-256) in the work directory and compiles with it
against glibc 2.28. Pass `--micromamba` if it is not on `PATH`, or `--toolchain`
to keep the toolchain elsewhere.

### What the script checks

- the toolchain matches its pins;
- tblite links exactly the recorded subprojects;
- every runtime dependency is either a system library (Windows: `KERNEL32`,
  `ADVAPI32`, the UCRT `api-ms-win-crt-*` sets; Linux: glibc) or a recorded
  runtime library with a known licence, and every bundled library finds its
  dependencies in `lib/`;
- all 117 C API functions are exported;
- Linux: no library needs a glibc symbol version newer than 2.28;
- it loads the bundle in a fresh process without any library search path, calls
  `tblite_get_version`, checks that every bundled library came from `lib/`, and
  that OpenBLAS selects its kernels at run time (`DYNAMIC_ARCH`).

## Running against a bundle

- **Linux:** give the executable the RUNPATH `$ORIGIN/lib`, for example
  `RUSTFLAGS='-C link-arg=-Wl,-rpath,$ORIGIN/lib'`, or set `LD_LIBRARY_PATH`. The
  executable's own `libgcc_s.so.1` then also resolves to the bundled one.
- **Windows:** the loader looks for a DLL's dependencies in the executable's
  directory and on `PATH`, not in the DLL's directory. Place the DLLs next to
  the executable, or delay-load `libtblite-0.dll` and register `lib\` with
  `SetDefaultDllDirectories` and `AddDllDirectory` before the first tblite call.
- **Threads:** `OMP_NUM_THREADS` sets tblite's OpenMP threads and
  `OPENBLAS_NUM_THREADS` OpenBLAS's. Windows' OpenBLAS uses OpenMP
  (`MAX_THREADS=64`), Linux's its own threads (`MAX_THREADS=128`). Set both.
- **CPUs:** the libraries target the x86-64 baseline; OpenBLAS picks optimized
  kernels for the CPU it runs on.

For tests, set `TBLITE_DIR` to the bundle and put `lib/` on `PATH` (Windows) or
`LD_LIBRARY_PATH` (Linux).

## Licences and obligations

| Component | Licence | Where in the bundle |
| --- | --- | --- |
| tblite, dftd4, s-dftd3, ddX | LGPL-3.0-or-later | compiled into the tblite library |
| multicharge, mctc-lib | Apache-2.0 | compiled into the tblite library |
| toml-f, jonquil | Apache-2.0 OR MIT | compiled into the tblite library |
| libquadmath | LGPL-2.1-or-later | own shared library |
| libgcc_s, libgfortran, libgomp | GPL-3.0-or-later WITH GCC-exception-3.1 | own shared libraries |
| OpenBLAS (with its LAPACK) | BSD-3-Clause | own shared library |
| winpthreads (Windows) | MIT and BSD-3-Clause | own shared library |

Every text is in `licenses/`; GCC's are vendored in `tools/bundle/licenses/gcc`
because the conda-forge packages carry only the exception. An application that
ships a bundle must, among others:

- ship the licence texts and `SOURCES.md`, and state prominently that it uses
  tblite under the LGPL;
- keep tblite and libquadmath replaceable shared libraries, which the layout does
  (LGPL-3.0 section 4(d)(1), LGPL-2.1 section 6(b));
- permit, in its licence terms, modification of these libraries for the user's
  own use and reverse engineering for debugging such modifications
  (LGPL-3.0 section 4);
- keep the corresponding source of the LGPL components available to recipients:
  the tblite release archive with the listed dependency commits and the visibility
  fix, and the GCC source behind libquadmath (`SOURCES.md` lists both, with the
  MSYS2 and conda-forge build recipes).

These notes describe the bundle's licences; they are not legal advice.

## Updating the pins

- **Linux:** solve again from the specs in the header of
  `tools/bundle/conda-linux-64.txt` and replace its package lines with the new
  environment's `URL#sha256:HASH` lines.
- **Windows:** install the new package versions in MSYS2 and replace their lines
  in `tools/bundle/msys2-ucrt64.txt` with the version and the SHA-256 of the
  package file (in `/var/cache/pacman/pkg` after installation).

Then build both bundles, run the tests against them, and review the changes to
`SOURCES.md`.
