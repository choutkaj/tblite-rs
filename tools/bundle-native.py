"""Build a redistributable tblite runtime bundle (Python 3.10+).

The bundle is the shared tblite library with every non-system library it
loads, their licence texts and SOURCES.md:

    include/       C headers (build time)
    lib/           runtime libraries, plus tblite.lib (MSVC) and pkgconfig/
    licenses/      licence texts, one directory per component
    SOURCES.md     versions, checksums, build settings and source locations

Windows x86-64 builds with the MSYS2 UCRT64 packages pinned in
tools/bundle/msys2-ucrt64.txt; Linux x86-64 with the conda-forge toolchain
pinned in tools/bundle/conda-linux-64.txt, against glibc 2.28; macOS arm64 with
the conda-forge toolchain pinned in tools/bundle/conda-osx-arm64.txt, for
macOS 11. tblite and its dependencies come from tools/build-native.py. See
docs/distribution.md.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[1]
PINS = ROOT / "tools/bundle"
# Load the pins of build-native.py without writing bytecode into the checkout.
sys.dont_write_bytecode = True
_spec = importlib.util.spec_from_file_location("build_native", ROOT / "tools/build-native.py")
native = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(native)
VERSION = native.VERSION
SERIES = VERSION.rsplit(".", 1)[0] + ".x"

GLIBC = (2, 28)
# The first macOS release for Apple silicon, and conda-forge's osx-arm64 target.
MACOS = (11, 0)
# tblite and the subprojects compiled into its library, with their licences
# and the files that state them. mstore and test-drive build only tests.
COMPILED = {
    "tblite": ("LGPL-3.0-or-later", ["COPYING", "COPYING.LESSER"]),
    "dftd4": ("LGPL-3.0-or-later", ["COPYING", "COPYING.LESSER"]),
    "s-dftd3": ("LGPL-3.0-or-later", ["COPYING", "COPYING.LESSER"]),
    "ddx": ("LGPL-3.0-or-later", ["LICENSE.txt"]),
    "multicharge": ("Apache-2.0", ["LICENSE"]),
    "mctc-lib": ("Apache-2.0", ["LICENSE"]),
    "toml-f": ("Apache-2.0 OR MIT", ["LICENSE-Apache", "LICENSE-MIT"]),
    "jonquil": ("Apache-2.0 OR MIT", ["LICENSE-Apache", "LICENSE-MIT"]),
}
GCC = "GPL-3.0-or-later WITH GCC-exception-3.1"
# Every runtime library a bundle may ship, with its licence and the licences/
# directory holding the texts. A new dependency fails the build until listed.
RUNTIME = {
    "libgcc_s.so.1": (GCC, "gcc"), "libgcc_s_seh-1.dll": (GCC, "gcc"), "libgcc_s.1.1.dylib": (GCC, "gcc"),
    "libgfortran.so.5": (GCC, "gcc"), "libgfortran-5.dll": (GCC, "gcc"), "libgfortran.5.dylib": (GCC, "gcc"),
    "libgomp.so.1": (GCC, "gcc"), "libgomp-1.dll": (GCC, "gcc"),
    "libquadmath.so.0": ("LGPL-2.1-or-later", "gcc"), "libquadmath-0.dll": ("LGPL-2.1-or-later", "gcc"),
    "libquadmath.0.dylib": ("LGPL-2.1-or-later", "gcc"),
    "libomp.dylib": ("Apache-2.0 WITH LLVM-exception", "llvm-openmp"),
    "libopenblas.so.0": ("BSD-3-Clause", "openblas"), "libopenblas.dll": ("BSD-3-Clause", "openblas"),
    "libopenblas.0.dylib": ("BSD-3-Clause", "openblas"),
    "libwinpthread-1.dll": ("MIT AND BSD-3-Clause", "winpthreads"),
}
# GCC's licence texts are vendored: conda-forge ships only the exception.
GCC_LICENSES = ["COPYING3", "COPYING.RUNTIME", "COPYING.LIB"]
FLAGS = {"CFLAGS", "CXXFLAGS", "FFLAGS", "FCFLAGS", "CPPFLAGS", "LDFLAGS", "PKG_CONFIG_PATH"}
LOADER = {"LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES"}
# Loads the bundle without a library search path and reports which bundle
# libraries the process holds, apart from any it held before (on Linux the
# interpreter may already hold the system libgcc_s, which then serves both).
SMOKE = r"""
import ctypes, json, os, sys
lib, names = sys.argv[1], sys.argv[2:]
bundled = [name for name in os.listdir(lib) if not os.path.islink(os.path.join(lib, name))
           and (".so." in name or name.endswith((".dll", ".dylib")))]
def held():
    if os.name == "nt":
        kernel32 = ctypes.WinDLL("kernel32")
        kernel32.GetModuleHandleW.restype = ctypes.c_void_p
        buffer, paths = ctypes.create_unicode_buffer(32768), {}
        for name in bundled:
            handle = kernel32.GetModuleHandleW(name)
            if handle:
                kernel32.GetModuleFileNameW(ctypes.c_void_p(handle), buffer, 32768)
                paths[name] = buffer.value
        return paths
    if sys.platform == "darwin":
        dyld = ctypes.CDLL(None)
        dyld._dyld_get_image_name.restype = ctypes.c_char_p
        files = {os.fsdecode(dyld._dyld_get_image_name(i)) for i in range(dyld._dyld_image_count())}
    else:
        with open("/proc/self/maps") as maps:
            files = {line.split()[-1] for line in maps if "/" in line}
    return {name: path for path in files for name in bundled if os.path.basename(path) == name}
before = held()
tblite, blas = (ctypes.CDLL(os.path.join(lib, name)) for name in names)
blas.openblas_get_config.restype = blas.openblas_get_corename.restype = ctypes.c_char_p
after = {name: path for name, path in held().items() if name not in before}
print(json.dumps({"version": tblite.tblite_get_version(), "config": blas.openblas_get_config().decode(),
                  "core": blas.openblas_get_corename().decode(), "bundled": bundled,
                  "before": sorted(before), "loaded": after}))
"""
# OpenBLAS's kernel selection: the word its configuration must contain, and a note.
DYNAMIC_ARCH = ("DYNAMIC_ARCH", "it selects kernels for the CPU at run time (`DYNAMIC_ARCH`; {core} on the "
                "build machine)")

def run(*args, **kwargs):
    print("+", " ".join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), check=True, **kwargs)

def output(*args):
    return subprocess.run(list(map(str, args)), check=True, capture_output=True, text=True).stdout

def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def git_revision():
    try:
        head = output("git", "-C", ROOT, "rev-parse", "HEAD").strip()
        dirty = output("git", "-C", ROOT, "status", "--porcelain", "--untracked-files=no").strip()
        return f"`{head}`" + (" with uncommitted changes" if dirty else "")
    except (OSError, subprocess.CalledProcessError):
        return "an unknown revision"

def without(names):
    return {k: v for k, v in os.environ.items() if k not in names}

class Conda:
    """A conda-forge toolchain, created from its lock in tools/bundle."""

    def __init__(self, args, work):
        self.prefix = (args.toolchain or work / "toolchain").resolve()
        lock = PINS / self.lock
        pinned = {line.split("#")[0] for line in lock.read_text().splitlines() if line.startswith("https://")}
        if not (self.prefix / "conda-meta").is_dir():
            micromamba = args.micromamba or shutil.which("micromamba") or os.environ.get("MAMBA_EXE")
            if not micromamba:
                raise SystemExit("Install micromamba, or pass --micromamba, to create the pinned toolchain")
            run(micromamba, "create", "--no-rc", "--yes", "--root-prefix", work / "mamba",
                "--prefix", self.prefix, "--file", lock)
        self.packages = [json.loads(p.read_text()) for p in (self.prefix / "conda-meta").glob("*.json")]
        if {p["url"] for p in self.packages} != pinned:
            raise SystemExit(f"{self.prefix} differs from {lock}; use a fresh --toolchain")

    def tool(self, name):
        return self.prefix / "bin" / f"{self.host}-{name}"

    def locate(self, name):
        path = self.prefix / "lib" / name
        if not path.exists():
            raise RuntimeError(f"{name} is not in {path.parent}")
        return path.resolve()

    def owner(self, path):
        relative = path.relative_to(self.prefix).as_posix()
        package = next(p for p in self.packages if relative in p["files"])
        info = Path(package["extracted_package_dir"]) / "info"
        extra = json.loads((info / "about.json").read_text()).get("extra", {})
        return {"package": f"conda-forge {package['name']} {package['version']} {package['build']}",
                "url": package["url"], "sha256": package["sha256"], "source": recipe_source(info / "recipe"),
                "recipe": f"{extra['remote_url']} at `{extra['sha']}`", "info": info}

    def licenses(self, component, owners):
        info = next(o["info"] for name, o in owners.items() if RUNTIME[name][1] == component)
        if component == "openblas":
            return {"LICENSE": info / "licenses/LICENSE", "LICENSE-lapack": info / "licenses/lapack-netlib/LICENSE"}
        if component == "llvm-openmp":
            return {"LICENSE.TXT": info / "licenses/openmp/LICENSE.TXT"}
        raise KeyError(component)

    def pinned(self):
        return (f"`tools/bundle/{self.lock}`: {len(self.packages)} conda-forge packages, each pinned by URL "
                "and SHA-256")

class Linux(Conda):
    name = "linux-x86_64"
    description = f"Linux x86-64 with glibc {GLIBC[0]}.{GLIBC[1]} or later"
    host = "x86_64-conda-linux-gnu"
    lock = "conda-linux-64.txt"
    blas = DYNAMIC_ARCH
    # glibc's libraries; libmvec holds the vector maths GCC calls in vectorized loops.
    system = {"libc.so.6", "libm.so.6", "libmvec.so.1", "libdl.so.2", "libpthread.so.0", "librt.so.1",
              "ld-linux-x86-64.so.2"}

    def environment(self):
        lib = self.prefix / "lib"
        env = without(FLAGS)
        env.update(PATH=f"{self.prefix / 'bin'}{os.pathsep}{env['PATH']}", FC=str(self.tool("gfortran")),
                   CC=str(self.tool("gcc")), CXX=str(self.tool("g++")), AR=str(self.tool("ar")),
                   PKG_CONFIG_LIBDIR=str(lib / "pkgconfig"), LDFLAGS=f"-L{lib} -Wl,-rpath-link,{lib}")
        return env

    def is_system(self, name):
        return name in self.system

    def dependencies(self, path):
        return re.findall(r"\(NEEDED\)\s+Shared library: \[(.+?)\]", output(self.tool("readelf"), "-d", path))

    def library(self, install, lib):
        shutil.copyfile((install / "lib/libtblite.so.0").resolve(), lib / "libtblite.so.0")
        (lib / "libtblite.so").symlink_to("libtblite.so.0")
        return lib / "libtblite.so.0"

    def finish(self, lib):
        """Point every library at its siblings, then check glibc and exports."""
        newest = (0, 0)
        for path in sorted(lib.glob("*.so.*")):
            run(self.prefix / "bin/patchelf", "--set-rpath", "$ORIGIN", path)
            for major, minor in re.findall(r"GLIBC_(\d+)\.(\d+)", output(self.tool("objdump"), "-T", path)):
                newest = max(newest, (int(major), int(minor)))
        if newest > GLIBC:
            raise RuntimeError(f"The bundle needs GLIBC_{newest[0]}.{newest[1]}, newer than "
                               f"{GLIBC[0]}.{GLIBC[1]}")
        symbols = output(self.tool("nm"), "-D", "--defined-only", lib / "libtblite.so.0")
        check_exports(re.findall(r" T (tblite_\w+)$", symbols, re.M))
        return [f"The newest glibc symbol version any library needs is GLIBC_{newest[0]}.{newest[1]}. "
                "Each library's RUNPATH is `$ORIGIN`, so it finds the others in `lib/`; this is the only "
                "change to the conda-forge binaries."]

    def toolchain(self):
        compiler = output(self.tool("gfortran"), "--version").splitlines()[0]
        sysroot = next(p for p in self.packages if p["name"] == "sysroot_linux-64")
        return [f"Compilers: {compiler}, conda-forge `{self.host}`, against the glibc {sysroot['version']} "
                f"sysroot (`sysroot_linux-64 {sysroot['version']} {sysroot['build']}`). The toolchain is "
                f"{self.pinned()}."]

class Macos(Conda):
    name = "macos-aarch64"
    description = f"macOS {MACOS[0]}.{MACOS[1]} and later on Apple silicon (arm64)"
    host = "arm64-apple-darwin20.0.0"
    lock = "conda-osx-arm64.txt"
    # conda-forge builds OpenBLAS for Apple silicon only, which covers every Apple M processor.
    blas = ("VORTEX", "it uses its kernels for Apple silicon (`VORTEX`; {core} on the build machine)")
    DYLIBS = {"LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LAZY_LOAD_DYLIB",
              "LC_LOAD_UPWARD_DYLIB"}

    def __init__(self, args, work):
        super().__init__(args, work)
        self.sdk = output("xcrun", "--sdk", "macosx", "--show-sdk-path").strip()

    def environment(self):
        lib = self.prefix / "lib"
        env = without(FLAGS | LOADER | {"LIBRARY_PATH", "CPATH", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET"})
        env.update(PATH=f"{self.prefix / 'bin'}{os.pathsep}{env['PATH']}", FC=str(self.tool("gfortran")),
                   CC=str(self.tool("clang")), CXX=str(self.tool("clang++")), AR=str(self.tool("ar")),
                   PKG_CONFIG_LIBDIR=str(lib / "pkgconfig"), SDKROOT=self.sdk,
                   MACOSX_DEPLOYMENT_TARGET=f"{MACOS[0]}.{MACOS[1]}",
                   LDFLAGS=f"-L{lib} -isysroot {self.sdk} -Wl,-headerpad_max_install_names")
        return env

    def is_system(self, name):
        return name.startswith(("/usr/lib/", "/System/Library/"))

    def load_commands(self, path):
        """The Mach-O load commands of the arm64 code in `path`, from `otool -l`."""
        commands = []
        for line in output(self.tool("otool"), "-arch", "arm64", "-l", path).splitlines():
            words = line.split()
            if words[:2] == ["Load", "command"]:
                commands.append({})
            elif commands and len(words) > 1:
                commands[-1].setdefault(words[0], re.sub(r" \(offset \d+\)$", "", " ".join(words[1:])))
        return commands

    def linked(self, path):
        return [c["name"] for c in self.load_commands(path) if c.get("cmd") in self.DYLIBS]

    def rpaths(self, path):
        return [c["path"] for c in self.load_commands(path) if c.get("cmd") == "LC_RPATH"]

    def dependencies(self, path):
        return [name if self.is_system(name) else Path(name).name for name in self.linked(path)]

    def library(self, install, lib):
        shutil.copyfile((install / "lib/libtblite.0.dylib").resolve(), lib / "libtblite.0.dylib")
        (lib / "libtblite.dylib").symlink_to("libtblite.0.dylib")
        return lib / "libtblite.0.dylib"

    def finish(self, lib):
        """Make the libraries find each other in `lib/`, re-sign them, then check them."""
        libraries = sorted(p for p in lib.glob("*.dylib") if not p.is_symlink())
        names = {p.name for p in libraries}
        install_name_tool = self.tool("install_name_tool")
        for path in libraries:
            changes = ["-id", f"@rpath/{path.name}"]
            for name in self.linked(path):
                if not self.is_system(name) and name != f"@rpath/{Path(name).name}":
                    changes += ["-change", name, f"@rpath/{Path(name).name}"]
            run(install_name_tool, *changes, path)
            while rpaths := self.rpaths(path):
                run(install_name_tool, *[arg for rpath in dict.fromkeys(rpaths) for arg in ("-delete_rpath", rpath)],
                    path)
            run(install_name_tool, "-add_rpath", "@loader_path", path)
            run("/usr/bin/codesign", "--force", "--sign", "-", path)
        newest, problems = (0, 0), []
        for path in libraries:
            commands = self.load_commands(path)
            ident = [c["name"] for c in commands if c.get("cmd") == "LC_ID_DYLIB"]
            if ident != [f"@rpath/{path.name}"]:
                problems.append(f"{path.name}: install name {ident}")
            if self.rpaths(path) != ["@loader_path"]:
                problems.append(f"{path.name}: rpaths {self.rpaths(path)}")
            # otool -L lists the install name and every linked library.
            for line in output(self.tool("otool"), "-arch", "arm64", "-L", path).splitlines()[1:]:
                name = line.strip().split(" (compatibility version")[0]
                if not (self.is_system(name) or (name.startswith("@rpath/") and name[7:] in names)):
                    problems.append(f"{path.name}: links {name}")
            versions = [c.get("minos") for c in commands if c.get("cmd") == "LC_BUILD_VERSION"]
            versions += [c.get("version") for c in commands if c.get("cmd") == "LC_VERSION_MIN_MACOSX"]
            if len(versions) != 1:
                problems.append(f"{path.name}: minimum macOS versions {versions}")
            else:
                newest = max(newest, tuple(int(part) for part in versions[0].split(".")[:2]))
            architectures = output(self.tool("lipo"), "-info", path).rsplit(": ", 1)[-1].split()
            if architectures != ["arm64"]:
                problems.append(f"{path.name}: architectures {architectures}")
            run("/usr/bin/codesign", "--verify", "--strict", path)
        if newest > MACOS:
            problems.append(f"a library needs macOS {newest[0]}.{newest[1]}, newer than {MACOS[0]}.{MACOS[1]}")
        if problems:
            raise RuntimeError("The bundle is not relocatable:\n  " + "\n  ".join(problems))
        symbols = output(self.tool("nm"), "-gU", lib / "libtblite.0.dylib")
        check_exports(re.findall(r" T _(tblite_\w+)$", symbols, re.M))
        return [f"The newest minimum macOS any library declares is {newest[0]}.{newest[1]}. Each library's "
                "install name is `@rpath/` and its file name, and its only rpath is `@loader_path`, so it finds "
                "the others in `lib/`. `install_name_tool` set these, and `codesign --force -s -` then signed "
                "each library ad hoc; these are the only changes to the conda-forge binaries. The bundle has no "
                "other signature: an application signs the libraries with its own identity."]

    def toolchain(self):
        fortran = output(self.tool("gfortran"), "--version").splitlines()[0]
        clang = output(self.tool("clang"), "--version").splitlines()[0]
        sdk = output("xcrun", "--sdk", "macosx", "--show-sdk-version").strip()
        return [f"Compilers: {fortran} and {clang}, conda-forge `{self.host}`, with the macOS {sdk} SDK of the "
                f"build machine and `MACOSX_DEPLOYMENT_TARGET={MACOS[0]}.{MACOS[1]}`. The toolchain is "
                f"{self.pinned()}."]

class Windows:
    name = "windows-x86_64"
    description = "Windows 10 and later, x86-64"
    blas = DYNAMIC_ARCH
    system = {"kernel32.dll", "advapi32.dll"}  # and the api-ms-win-crt-* sets of the Universal C Runtime

    def __init__(self, args, work):
        self.root = Path(args.msys2 or os.environ.get("MSYS2_ROOT") or r"C:\msys64").resolve()
        self.ucrt = self.root / "ucrt64"
        self.pacman = self.root / "usr/bin/pacman.exe"
        if not self.pacman.is_file():
            raise SystemExit(f"No MSYS2 installation at {self.root}; pass --msys2")
        self.pins = msys2_pins()
        wrong = []
        for name, version, _ in self.pins:
            installed = subprocess.run([self.pacman, "-Q", name], capture_output=True, text=True).stdout.split()
            if installed[1:] != [version]:
                wrong.append(f"{name} {version} (installed: {' '.join(installed[1:]) or 'none'})")
        if wrong:
            urls = " ".join(msys2_url(name, version) for name, version, _ in self.pins)
            raise SystemExit("MSYS2 lacks the pinned packages:\n  " + "\n  ".join(wrong) +
                             f"\nInstall them in a UCRT64 shell with:\n  pacman -U --needed {urls}")

    def environment(self):
        env = without(FLAGS)
        env.update(PATH=f"{self.ucrt / 'bin'}{os.pathsep}{env['PATH']}", FC="gfortran", CC="gcc", CXX="g++")
        return env

    def is_system(self, name):
        return name.lower() in self.system or name.lower().startswith("api-ms-win-crt-")

    def dependencies(self, path):
        return re.findall(r"DLL Name: (\S+)", output(self.ucrt / "bin/objdump.exe", "-p", path))

    def locate(self, name):
        path = self.ucrt / "bin" / name
        if not path.is_file():
            raise RuntimeError(f"{name} is not in {path.parent}")
        return path

    def library(self, install, lib):
        run("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            ROOT / "tools/windows-import-library.ps1", "-Prefix", install)
        dll = next((install / "bin").glob("libtblite*.dll"))
        shutil.copyfile(install / "lib/tblite.lib", lib / "tblite.lib")
        shutil.copyfile(dll, lib / dll.name)
        return lib / dll.name

    def finish(self, lib):
        return ["`tblite.lib` is the MSVC import library, generated from the DLL by "
                "`tools/windows-import-library.ps1`, which also checks every C API export."]

    def owner(self, path):
        # pacman takes the path inside the MSYS2 root.
        package, version = output(self.pacman, "-Qo", "/" + path.relative_to(self.root).as_posix()).split()[-2:]
        pin = next(p for p in self.pins if p[0] == package)
        if pin[1] != version:
            raise RuntimeError(f"{package} {version} is not the pinned {pin[1]}")
        desc = (self.root / f"var/lib/pacman/local/{package}-{version}/desc").read_text().splitlines()
        base = desc[desc.index("%BASE%") + 1]
        return {"package": f"MSYS2 {package} {version}", "url": msys2_url(package, version), "sha256": pin[2],
                "source": f"https://repo.msys2.org/mingw/sources/{base}-{version}.src.tar.zst",
                "recipe": f"https://github.com/msys2/MINGW-packages/tree/master/{base}"}

    def licenses(self, component, owners):
        licenses = self.ucrt / "share/licenses"
        if component == "openblas":
            return {"LICENSE": licenses / "openblas/LICENSE", "LICENSE-lapack": licenses / "openblas/LICENSE-lapack"}
        if component == "winpthreads":
            return {"COPYING": licenses / "libwinpthread/COPYING"}
        raise KeyError(component)

    def toolchain(self):
        compiler = output(self.ucrt / "bin/gfortran.exe", "--version").splitlines()[0]
        return [f"Compilers: {compiler}, MSYS2 UCRT64, linked against the Universal C Runtime. The toolchain "
                f"is `tools/bundle/msys2-ucrt64.txt`: {len(self.pins)} MSYS2 packages, each pinned by version and "
                "SHA-256."]

TARGETS = {"win32": (Windows, {"amd64", "x86_64"}), "linux": (Linux, {"x86_64"}), "darwin": (Macos, {"arm64"})}

def msys2_pins():
    """(name, version, SHA-256) of every pinned MSYS2 package."""
    return [line.split() for line in (PINS / "msys2-ucrt64.txt").read_text().splitlines()
            if line and not line.startswith("#")]

def msys2_url(name, version):
    return f"https://repo.msys2.org/mingw/ucrt64/{name}-{version}-any.pkg.tar.zst"

def recipe_source(recipe):
    """The first source URL and SHA-256 of a conda package's rendered recipe."""
    for name in ("rendered_recipe.yaml", "meta.yaml"):
        path = recipe / name
        if path.is_file():
            text = path.read_text(encoding="utf-8")
            section = text[text.index("source:"):] if "source:" in text else ""
            url = re.search(r"(https?://\S+)", section)
            digest = re.search(r"sha256:\s*([0-9a-f]{64})", section)
            if url:
                return f"{url.group(1)} (SHA-256 `{digest.group(1)}`)" if digest else url.group(1)
    raise RuntimeError(f"No source URL in {recipe}")

def check_exports(exported):
    expected = {f["name"] for f in json.loads((ROOT / "tblite-sys/api.json").read_text())["functions"]}
    if expected - set(exported):
        raise RuntimeError(f"Missing exports: {sorted(expected - set(exported))}")

def compiled_subprojects(build):
    """Names of the subproject archives linked into the tblite library."""
    lines, names, inside = (build / "build.ninja").read_text(encoding="utf-8").splitlines(), set(), False
    for line in lines:
        if line.startswith("build "):
            inside = line.startswith("build libtblite") and "_LINKER" in line
        if inside:
            names.update(re.findall(r"subprojects/([\w.-]+)/lib[\w.-]+\.a", line))
    return names

def write_pc(lib):
    (lib / "pkgconfig").mkdir()
    (lib / "pkgconfig/tblite.pc").write_text(
        "prefix=${pcfiledir}/../..\nincludedir=${prefix}/include\nlibdir=${prefix}/lib\n\n"
        f"Name: tblite\nDescription: Light-weight tight-binding framework\nVersion: {VERSION}\n"
        "Libs: -L${libdir} -ltblite\nCflags: -I${includedir}\n", encoding="utf-8", newline="\n")

def archive(out, name, directory):
    """Pack the bundle as tblite-VERSION-NAME.tar.gz, with NAME/ as its top-level folder."""
    def anonymous(info):
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        return info
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"tblite-{VERSION}-{name}.tar.gz"
    with tarfile.open(path, "w:gz") as tar:
        tar.add(out, arcname=name, filter=anonymous)
    return path

def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, help="new bundle directory")
    parser.add_argument("--archive", type=Path, metavar="DIR",
                        help="also write tblite-VERSION-PLATFORM.tar.gz into DIR")
    parser.add_argument("--work-dir", type=Path, default=ROOT / ".native/bundle",
                        help="sources, build tree and, on Linux and macOS, the toolchain")
    parser.add_argument("--toolchain", type=Path, help="Linux and macOS: conda prefix to create or reuse")
    parser.add_argument("--micromamba", help="Linux and macOS: micromamba executable")
    parser.add_argument("--msys2", help=r"Windows: MSYS2 root (default: MSYS2_ROOT or C:\msys64)")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--msys2-urls", action="store_true",
                        help="print the pinned MSYS2 package URLs, for pacman -U, and exit")
    args = parser.parse_args()
    if args.msys2_urls:
        print("\n".join(msys2_url(name, version) for name, version, _ in msys2_pins()))
        return
    if not args.out:
        parser.error("--out is required")
    target_class, machines = TARGETS.get(sys.platform, (None, set()))
    if platform.machine().lower() not in machines:
        parser.error("bundles are built on Windows and Linux x86-64 and on macOS arm64")
    out = args.out.resolve()
    if out.exists() and any(out.iterdir()):
        parser.error(f"{out} is not empty")
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    target = target_class(args, work)
    install, build, source = work / "install", work / "build", work / "source"
    if install.exists():
        shutil.rmtree(install)
    run(sys.executable, ROOT / "tools/build-native.py", "--prefix", install, "--work-dir", source,
        "--build-dir", build, "--library", "shared", "--hdf5", "disabled", "--jobs", args.jobs,
        env=target.environment())
    if compiled_subprojects(build) != set(COMPILED) - {"tblite"}:
        raise RuntimeError(f"tblite links {sorted(compiled_subprojects(build))}, not the recorded subprojects")
    tree = source / f"tblite-{VERSION}"

    lib = out / "lib"
    (out / "include/tblite").mkdir(parents=True)
    lib.mkdir()
    for header in [install / "include/tblite.h", *(install / "include/tblite").glob("*.h")]:
        shutil.copyfile(header, out / header.relative_to(install))
    library = target.library(install, lib)
    write_pc(lib)
    pending, owners = [library], {}
    while pending:
        for dependency in target.dependencies(pending.pop()):
            if dependency in owners or target.is_system(dependency):
                continue
            if dependency not in RUNTIME:
                raise RuntimeError(f"Unexpected runtime dependency {dependency}; record its licence first")
            path = target.locate(dependency)
            owners[dependency] = target.owner(path)
            shutil.copyfile(path, lib / dependency)
            pending.append(lib / dependency)
    notes = target.finish(lib)

    licenses = out / "licenses"
    for name, (_, files) in COMPILED.items():
        (licenses / name).mkdir(parents=True)
        for filename in files:
            shutil.copyfile(tree / ("" if name == "tblite" else f"subprojects/{name}") / filename,
                            licenses / name / filename)
    (licenses / "gcc").mkdir()
    for filename in GCC_LICENSES:
        shutil.copyfile(PINS / "licenses/gcc" / filename, licenses / "gcc" / filename)
    for component in sorted({RUNTIME[name][1] for name in owners} - {"gcc"}):
        (licenses / component).mkdir()
        for filename, origin in target.licenses(component, owners).items():
            shutil.copyfile(origin, licenses / component / filename)

    blas = next(name for name in owners if name.startswith("libopenblas"))
    env = without(LOADER | {"PATH"}) | {"PATH": os.path.join(os.environ.get("SystemRoot", "/usr"),
                                                             "System32" if os.name == "nt" else "bin")}
    smoke = subprocess.run([sys.executable, "-I", "-c", SMOKE, lib, library.name, blas],
                           capture_output=True, text=True, env=env)
    if smoke.returncode:
        raise RuntimeError(f"The bundle does not load:\n{smoke.stderr}")
    loaded = json.loads(smoke.stdout)
    code = sum(int(part) * 100 ** i for i, part in enumerate(reversed(VERSION.split("."))))
    if loaded["version"] != code or target.blas[0] not in loaded["config"].split():
        raise RuntimeError(f"Unexpected tblite version or OpenBLAS build: {loaded}")
    outside = {name: path for name, path in loaded["loaded"].items() if Path(path).resolve().parent != lib.resolve()}
    if outside or set(loaded["loaded"]) | set(loaded["before"]) != set(loaded["bundled"]):
        raise RuntimeError(f"The bundle's libraries did not all load from {lib}: {loaded}")
    write_sources(out, target, owners, loaded, notes)
    print(f"Bundle for {target.name} in {out}")
    if args.archive:
        print(f"Archive: {archive(out, target.name, args.archive.resolve())}")

def write_sources(out, target, owners, loaded, notes):
    lines = [f"# tblite {VERSION} runtime bundle, {target.name}", "",
             f"tblite {VERSION} as a shared library, with the runtime libraries it loads, for {target.description}. "
             f"Built by tblite-rs `tools/bundle-native.py` at {git_revision()}.", "",
             "Applications load these libraries at run time. Replacing them with an interface-compatible build, "
             f"tblite {SERIES} with its C API, runs the application with a modified tblite (LGPL-3.0 section "
             "4(d)(1)); this recipe rebuilds them from modified sources. The licence texts are in `licenses/`.",
             "", "## Files", "", "| File | Contents | Licence | SHA-256 |", "| --- | --- | --- | --- |"]
    for path in sorted(p for p in out.rglob("*") if p.is_file() and not p.is_symlink()):
        relative = path.relative_to(out).as_posix()
        if path.name in owners:
            contents, licence = owners[path.name]["package"], RUNTIME[path.name][0]
        elif relative.startswith("lib/") and "tblite" in path.name and not relative.endswith(".pc"):
            contents = f"tblite {VERSION} and the projects compiled into it" + (
                ", MSVC import library" if path.suffix == ".lib" else "")
            licence = "LGPL-3.0-or-later, with Apache-2.0 and MIT parts"
        elif relative.startswith("include/"):
            contents, licence = f"tblite {VERSION} C header", "LGPL-3.0-or-later"
        elif relative.startswith("licenses/"):
            contents, licence = "licence text", ""
        else:
            contents, licence = "pkg-config metadata", ""
        lines.append(f"| `{relative}` | {contents} | {licence} | `{sha256(path)}` |")
    lines += ["", "## tblite and the projects compiled into it", "",
              "`tools/build-native.py` builds these from source into the one tblite library:", "",
              "| Project | Version | Licence | Source |", "| --- | --- | --- | --- |",
              f"| tblite | {VERSION} | {COMPILED['tblite'][0]} | {native.URL} (SHA-256 `{native.SHA256}`) |"]
    for name, (licence, _) in COMPILED.items():
        if name != "tblite":
            url, revision = native.DEPS[name]
            lines.append(f"| {name} | commit `{revision}` | {licence} | {url.removesuffix('.git')} |")
    lines += ["", f"The commits are those the Meson wraps of the tblite {VERSION} release resolve to. One change is "
              f"applied to the release: the 13 released C entry points whose Fortran names are `PRIVATE` in {VERSION} "
              "are declared `PUBLIC` in `src/tblite/api/{calculator,container,double_dictionary,error,result}.f90`, "
              "so every platform's library exports them (GCC PR126872). No signature or procedure body changes; "
              "`patch_c_api_visibility` in `tools/build-native.py` is the exact edit.", "",
              "Build settings: Meson release build of the shared library, C API, OpenMP, OpenBLAS for BLAS and "
              "LAPACK, ddX; no HDF5 or TREXIO.", "", *target.toolchain(), "", *notes, "",
              "## Runtime libraries", "", "| File | Package | Package SHA-256 | Source | Build recipe |",
              "| --- | --- | --- | --- | --- |"]
    for name, owner in sorted(owners.items()):
        lines.append(f"| `{name}` | [{owner['package']}]({owner['url']}) | `{owner['sha256']}` | "
                     f"{owner['source']} | {owner['recipe']} |")
    components = {RUNTIME[name][1] for name in owners}
    gcc = [stem for stem in ("libgcc_s", "libgfortran", "libgomp") if any(n.startswith(stem) for n in owners)]
    lines += ["", f"OpenBLAS reports `{loaded['config']}`: " + target.blas[1].format(core=loaded["core"]) +
              ". `OMP_NUM_THREADS` sets tblite's threads and `OPENBLAS_NUM_THREADS` OpenBLAS's.", "",
              "## Licences", "",
              "- tblite, dftd4, s-dftd3 and ddX: GNU LGPL 3.0 or later (`COPYING.LESSER`, ddX `LICENSE.txt`), "
              "which supplements the GNU GPL 3.0 (`tblite/COPYING`).",
              "- multicharge and mctc-lib: Apache 2.0. toml-f and jonquil: Apache 2.0 or MIT.",
              f"- GCC runtime libraries ({', '.join(gcc)}): GNU GPL 3.0 or later (`gcc/COPYING3`) "
              "with the GCC Runtime Library Exception 3.1 (`gcc/COPYING.RUNTIME`).",
              "- libquadmath: GNU LGPL 2.1 or later (`gcc/COPYING.LIB`).",
              "- OpenBLAS: BSD 3-Clause, with the modified BSD licence of its LAPACK (`openblas/`)."]
    if "llvm-openmp" in components:
        lines.append("- LLVM OpenMP (libomp): Apache 2.0 with LLVM exceptions (`llvm-openmp/LICENSE.TXT`).")
    if "winpthreads" in components:
        lines.append("- winpthreads (libwinpthread): MIT, with BSD 3-Clause parts (`winpthreads/COPYING`).")
    lines += ["", "Distributing this bundle distributes the object code of its LGPL components. Their corresponding "
              "source, listed above, must stay available to recipients as LGPL 3.0 and LGPL 2.1 require.", ""]
    (out / "SOURCES.md").write_text("\n".join(lines), encoding="utf-8", newline="\n")

if __name__ == "__main__":
    main()
