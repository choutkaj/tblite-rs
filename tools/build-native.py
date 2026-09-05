"""Install a pinned tblite independently of Cargo (Python 3.10+).

Prerequisites: Meson, Ninja, Git, a Fortran/C/C++ toolchain, OpenBLAS and
optionally HDF5 with Fortran modules. See docs/installation.md.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SHA256 = "7864755e3faeef43a2f334a1679f6ece525eb604837e772c6623c042214ea39f"
URL = "https://codeload.github.com/tblite/tblite/tar.gz/refs/tags/v0.7.0"
# Commits resolved from the release's Meson wraps, including nested dependencies.
DEPS = {
    "toml-f": ("https://github.com/toml-f/toml-f", "51a26158c6d52bbc59cb482bdd13f00d7fd032a3"),
    "mctc-lib": ("https://github.com/grimme-lab/mctc-lib", "e9de066d89f250d1cfb6de3a33f0c27c0e2f855d"),
    "dftd4": ("https://github.com/dftd4/dftd4", "6e1f59c3f39d919a2dbef0601d2576727c8b30e8"),
    "s-dftd3": ("https://github.com/dftd3/simple-dftd3", "6f0b06fbfa8653a23ca55c453772ce3af4420706"),
    "multicharge": ("https://github.com/grimme-lab/multicharge", "6a5d63f9e9e29dcf13cc47cc27f33bf9015681bf"),
    "ddx": ("https://github.com/ddsolvation/ddX.git", "4d79e3d9caeae5e602683572a71cb550414f9b09"),
    "mstore": ("https://github.com/grimme-lab/mstore", "663245d739be0123da61c917e55116b0c3db4c74"),
    "jonquil": ("https://github.com/toml-f/jonquil", "4d43ffea512977602f654ab10067fcddb3e3c107"),
    "test-drive": ("https://github.com/fortran-lang/test-drive.git", "d16852743043963f294a5d9a3d5218e32c20ea7f"),
}
# These released C entry points have PRIVATE Fortran names in 0.7.0. GNU
# Fortran can hide their symbols on macOS (GCC PR126872). Explicit PUBLIC
# declarations preserve the C ABI without changing any procedure bodies.
C_API_PUBLIC = {
    "calculator.f90": (
        "set_calculator_guess_api", "get_calculator_shell_count",
        "get_calculator_shell_map", "get_calculator_angular_momenta",
        "get_calculator_orbital_count", "get_calculator_orbital_map",
    ),
    "container.f90": (
        "push_back_api", "new_electric_field_api", "new_spin_polarization_api",
    ),
    "double_dictionary.f90": ("delete_post_processing_api",),
    "error.f90": ("set_error_api",),
    "result.f90": ("save_result_wavefunction_api", "load_result_wavefunction_api"),
}

def patch_c_api_visibility(source, archive):
    """Apply the visibility fix against the already checksum-verified archive.

    Accept pristine or previously patched sources; reject other modifications
    instead of overwriting them. Keep the patch identical on every platform.
    """
    updates = []
    with tarfile.open(archive) as tar:
        for filename, procedures in C_API_PUBLIC.items():
            relative = Path("src/tblite/api") / filename
            with tar.extractfile(f"tblite-0.7.0/{relative.as_posix()}") as stream:
                original = stream.read().decode("utf-8").replace("\r\n", "\n")
            marker = "   private\n"
            if original.count(marker) != 1:
                raise RuntimeError(f"Unexpected module layout: {relative}")
            declarations = "\n   ! tblite-rs: keep released C entry points externally visible.\n"
            declarations += "".join(f"   public :: {name}\n" for name in procedures)
            patched = original.replace(marker, marker + declarations, 1)
            path = source / relative
            current = path.read_text(encoding="utf-8")
            if current not in (original, patched):
                raise RuntimeError(f"Native source differs: {relative}; use a fresh --work-dir")
            if current != patched:
                updates.append((path, patched))
    for path, patched in updates:
        path.write_text(patched, encoding="utf-8", newline="\n")
    print("Verified native C API visibility fix (13 entry points)", flush=True)

def run(*args, **kwargs):
    print("+", " ".join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, **kwargs)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, default=ROOT / ".native")
    parser.add_argument("--build-dir", type=Path)
    parser.add_argument("--library", choices=["shared", "static", "both"], default="shared")
    parser.add_argument("--minimal", action="store_true", help="Disable ddX and HDF5")
    parser.add_argument("--trexio", choices=["disabled", "enabled", "auto"], default="disabled")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    if os.name == "nt" and args.library != "shared":
        parser.error("Windows installation supports shared libraries only")
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    archive = work / "tblite-0.7.0.tar.gz"
    if not archive.exists():
        print(f"Downloading {URL}", flush=True)
        urllib.request.urlretrieve(URL, archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
        raise RuntimeError(f"Checksum mismatch: {archive}; remove the file and retry")
    source = work / "tblite-0.7.0"
    if not source.exists():
        with tarfile.open(archive) as tar:
            # The archive is verified; also restrict every extraction to this root.
            for member in tar.getmembers():
                if not (work / member.name).resolve().is_relative_to(source):
                    raise RuntimeError(f"Unexpected archive member: {member.name}")
            if hasattr(tarfile, "data_filter"):
                tar.extractall(work, filter="data")
            else:
                tar.extractall(work)
    patch_c_api_visibility(source, archive)
    for name, (url, revision) in DEPS.items():
        dep = source / "subprojects" / name
        if not dep.exists():
            run("git", "init", dep)
            run("git", "-C", dep, "fetch", "--depth=1", url, revision)
            run("git", "-C", dep, "checkout", "--detach", "FETCH_HEAD")
        head = subprocess.check_output(["git", "-C", str(dep), "rev-parse", "HEAD"], text=True).strip()
        if head != revision:
            raise RuntimeError(f"{dep}: expected {revision}, found {head}; use a fresh --work-dir")
    # Windows Git may materialize ddX's relative symlink as a text file.
    stub = source / "subprojects/ddx/tests/standalone_tests/ddx_driver_testing.f90"
    if not stub.is_symlink() and stub.read_text().strip() == "../../src/ddx_driver.f90":
        shutil.copyfile(source / "subprojects/ddx/src/ddx_driver.f90", stub)
    # Ensure reused sources still match our public ABI baseline.
    for header in (ROOT / "tblite-sys/vendor/tblite-0.7.0/include").rglob("*.h"):
        relative = header.relative_to(ROOT / "tblite-sys/vendor/tblite-0.7.0")
        if header.read_bytes().replace(b"\r\n", b"\n") != (source / relative).read_bytes().replace(b"\r\n", b"\n"):
            raise RuntimeError(f"Public header differs: {relative}")
    build = (args.build_dir or work / ("build-minimal" if args.minimal else "build")).resolve()
    command = ["meson", "setup", build, source, f"--prefix={args.prefix.resolve()}", "--libdir=lib",
               "--buildtype=release", f"--default-library={args.library}", "-Dapi=true", "-Dlapack=openblas",
               f"-Dddx={'false' if args.minimal else 'true'}", f"-Dhdf5={'disabled' if args.minimal else 'enabled'}",
               f"-Dtrexio={args.trexio}", "--wrap-mode=nodownload"]
    # Use the pinned local Fortran dependencies even if other versions are installed.
    command += ["--force-fallback-for=" + ",".join(["toml-f", "mctc-lib", "dftd4", "s-dftd3", "multicharge", "ddx", "mstore", "jonquil", "test-drive"])]
    if (build / "meson-private/coredata.dat").exists():
        command += ["--reconfigure"]
    if os.name == "nt":
        # Matches upstream's GNU Windows linker workaround.
        command += ["-Dfortran_link_args=-Wl,--allow-multiple-definition"]
    run(*command)
    run("meson", "compile", "-C", build, "-j", args.jobs)
    run("meson", "install", "-C", build)
    print(f"Installed tblite 0.7.0 at {args.prefix.resolve()}")

if __name__ == "__main__":
    main()
