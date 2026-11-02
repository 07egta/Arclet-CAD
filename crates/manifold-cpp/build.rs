//! Optional offline CMake build of Manifold. MANIFOLD_SRC is the checkout root.

use std::path::PathBuf;
use std::process::Command;

fn run(mut command: Command, step: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status().map_err(|err| format!("{step}: cannot start tool: {err}"))?;
    if !status.success() {
        return Err(format!("{step}: command exited with {status}").into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cpp/shim.cpp");
    println!("cargo:rerun-if-env-changed=MANIFOLD_SRC");
    if std::env::var_os("CARGO_FEATURE_MANIFOLD").is_none() {
        return Ok(());
    }

    let target = std::env::var("CARGO_CFG_TARGET_OS")?;
    if !matches!(target.as_str(), "windows" | "macos" | "linux") {
        return Err(format!("Manifold native bridge does not support target OS {target}").into());
    }
    let source = PathBuf::from(std::env::var_os("MANIFOLD_SRC")
        .ok_or("feature manifold requires MANIFOLD_SRC=<Manifold checkout root>; see docs/native-cpp-bridge.md")?);
    if !source.join("CMakeLists.txt").is_file() || !source.join("include/manifold/manifold.h").is_file() {
        return Err("MANIFOLD_SRC must be a complete Manifold C++ source checkout".into());
    }
    println!("cargo:rerun-if-changed={}", source.join("src").display());
    println!("cargo:rerun-if-changed={}", source.join("include").display());
    println!("cargo:rerun-if-changed={}", source.join("CMakeLists.txt").display());

    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("Cargo did not provide OUT_DIR")?);
    let build = out.join("manifold-build");
    let install = out.join("manifold-install");
    let mut config = Command::new("cmake");
    config.arg("-S").arg(&source).arg("-B").arg(&build)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display()))
        .args([
            "-DCMAKE_INSTALL_LIBDIR=lib",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DMANIFOLD_DOWNLOADS=OFF",
            "-DMANIFOLD_PAR=OFF",
            "-DMANIFOLD_TEST=OFF",
            "-DMANIFOLD_CBIND=OFF",
            "-DMANIFOLD_PYBIND=OFF",
            "-DMANIFOLD_JSBIND=OFF",
            "-DASSIMP_ENABLE=OFF",
            "-DTRACY_ENABLE=OFF",
        ]);
    run(config, "configuring Manifold (CMake)")?;
    let mut compile = Command::new("cmake");
    compile.arg("--build").arg(&build)
        .args(["--config", "Release", "--target", "manifold", "--parallel", "2"]);
    run(compile, "building Manifold (CMake)")?;
    let mut copy = Command::new("cmake");
    copy.arg("--install").arg(&build).args(["--config", "Release"]);
    run(copy, "installing Manifold (CMake)")?;

    let lib = install.join("lib");
    let static_name = if target == "windows" { "manifold.lib" } else { "libmanifold.a" };
    if !lib.join(static_name).is_file() {
        return Err(format!("Manifold did not produce {static_name} under {}", lib.display()).into());
    }
    // The C++ ABI stays inside cpp/shim.cpp; Rust sees extern C only.
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(source.join("include"))
        .include(build.join("include"))
        .file("cpp/shim.cpp")
        .compile("arclet_manifold_shim");
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=manifold");
    Ok(())
}
