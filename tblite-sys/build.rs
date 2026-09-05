use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=TBLITE_DIR");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=FC");
    println!("cargo:rerun-if-env-changed=TBLITE_FORTRAN_LIB_DIR");
    if env::var_os("DOCS_RS").is_some() {
        return;
    }
    let target = env::var("TARGET").unwrap();
    let static_link = env::var_os("CARGO_FEATURE_STATIC").is_some();
    assert!(!(static_link && target.contains("windows")), "Windows static linking is not supported. Remove the `static` feature and use a tblite DLL with an MSVC import library.");
    let prefix = env::var_os("TBLITE_DIR").map(PathBuf::from);
    let include;
    if let Some(ref root) = prefix {
        assert!(
            root.is_absolute(),
            "TBLITE_DIR must be an absolute installation prefix"
        );
        include = root.join("include");
        assert!(
            include.join("tblite.h").is_file(),
            "TBLITE_DIR must contain include/tblite.h; see docs/installation.md"
        );
        let pc = [root.join("lib/pkgconfig/tblite.pc"), root.join("lib64/pkgconfig/tblite.pc")].into_iter().find(|p| p.is_file()).expect("TBLITE_DIR must contain the tblite.pc installed by Meson (lib/pkgconfig or lib64/pkgconfig)");
        println!("cargo:rerun-if-changed={}", pc.display());
        let metadata = fs::read_to_string(&pc).expect("read tblite.pc");
        let version = metadata
            .lines()
            .find_map(|l| l.strip_prefix("Version:"))
            .expect("tblite.pc has no Version")
            .trim();
        check_version(version);
        if static_link {
            // Keep private/transitive libraries and link ordering from upstream metadata.
            let old = env::var_os("PKG_CONFIG_PATH").unwrap_or_default();
            let mut paths = vec![pc.parent().unwrap().to_path_buf()];
            paths.extend(env::split_paths(&old));
            env::set_var("PKG_CONFIG_PATH", env::join_paths(paths).unwrap());
            probe(true);
        } else {
            let libdir = pc.parent().unwrap().parent().unwrap();
            if target.contains("msvc") {
                assert!(libdir.join("tblite.lib").is_file(), "Missing tblite.lib. Run tools/windows-import-library.ps1 against the installed DLL; a GNU .dll.a is not the MSVC import library.");
            } else {
                let found = fs::read_dir(libdir)
                    .unwrap()
                    .filter_map(Result::ok)
                    .any(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        name.starts_with("libtblite")
                            && (name.contains(".so")
                                || name.ends_with(".dylib")
                                || name.ends_with(".dll.a"))
                    });
                assert!(
                    found,
                    "No shared tblite library found; use a shared build or enable `static`"
                );
            }
            println!("cargo:rustc-link-search=native={}", libdir.display());
            println!("cargo:rustc-link-lib=dylib=tblite");
        }
    } else {
        let lib = probe(static_link);
        include = lib
            .include_paths
            .into_iter()
            .find(|p| p.join("tblite.h").is_file())
            .expect("pkg-config did not locate tblite.h");
    }
    println!("cargo:include={}", include.display());
    #[cfg(feature = "abi-tests")]
    {
        println!("cargo:rerun-if-changed=tests/abi.c");
        cc::Build::new()
            .file("tests/abi.c")
            .include(&include)
            .flag_if_supported("-std=c11")
            .flag_if_supported("/std:c11")
            .compile("tblite_rs_abi");
    }
}

fn check_version(version: &str) {
    let mut parts = version.split('.');
    assert!(
        parts.next() == Some("0")
            && parts.next() == Some("7")
            && parts.next().and_then(|p| p.parse::<u32>().ok()).is_some(),
        "Unsupported tblite {version}; these bindings require 0.7.x"
    );
}

fn probe(static_link: bool) -> pkg_config::Library {
    let lib = pkg_config::Config::new().range_version("0.7.0".."0.8.0").statik(static_link).probe("tblite").unwrap_or_else(|e| panic!("Cannot link tblite 0.7.x: {e}\nInstall tblite separately and set PKG_CONFIG_PATH or TBLITE_DIR. See docs/installation.md."));
    check_version(&lib.version);
    if static_link {
        assert!(lib.link_paths.iter().any(|p| p.join("libtblite.a").is_file()), "The `static` feature requires libtblite.a; rebuild tblite with --default-library=both or static");
        // Meson's 0.7 metadata omits HDF5's Fortran module library and
        // pkg-config's Rust adapter drops the GNU driver flag -fopenmp.
        let pcdir = pkg_config::get_variable("tblite", "pcfiledir").expect("tblite.pc location");
        let pc =
            fs::read_to_string(PathBuf::from(pcdir).join("tblite.pc")).expect("read tblite.pc");
        if lib.libs.iter().any(|n| n == "gfortran") {
            let target = env::var("TARGET").unwrap();
            let runtime_dir = env::var_os("TBLITE_FORTRAN_LIB_DIR")
                .map(PathBuf::from)
                .or_else(|| {
                    if env::var("HOST").ok().as_ref() != Some(&target) {
                        return None;
                    }
                    let filename = if target.contains("apple") {
                        "libgfortran.dylib"
                    } else {
                        "libgfortran.so"
                    };
                    let output = std::process::Command::new(
                        env::var_os("FC").unwrap_or_else(|| "gfortran".into()),
                    )
                    .arg(format!("-print-file-name={filename}"))
                    .output()
                    .ok()?;
                    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
                    if output.status.success() && path.is_absolute() && path.is_file() {
                        path.parent().map(PathBuf::from)
                    } else {
                        None
                    }
                });
            if let Some(dir) = runtime_dir {
                println!("cargo:rustc-link-search=native={}", dir.display());
                // Homebrew's metadata can select libgfortran.a without its
                // quadmath dependency. Some GNU targets do not provide it.
                let extension = if target.contains("apple") {
                    "dylib"
                } else {
                    "so"
                };
                if !lib.libs.iter().any(|n| n == "quadmath")
                    && dir.join(format!("libquadmath.{extension}")).is_file()
                {
                    println!("cargo:rustc-link-lib=dylib=quadmath");
                }
            }
            if pc.split_whitespace().any(|s| s == "-fopenmp")
                && !lib.libs.iter().any(|n| n == "gomp")
            {
                println!("cargo:rustc-link-lib=dylib=gomp");
            }
        }
        if lib.libs.iter().any(|n| n == "hdf5") && !lib.libs.iter().any(|n| n == "hdf5_fortran") {
            println!("cargo:rustc-link-lib=dylib=hdf5_fortran");
        }
    }
    lib
}
