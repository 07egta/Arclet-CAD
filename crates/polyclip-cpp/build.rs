//! Builds the optional Clipper2 shim when the `clipper2` feature is on.
//!
//! Clipper2 is not vendored. Point `CLIPPER2_SRC` at the `CPP/Clipper2Lib` directory of a
//! checkout of https://github.com/AngusJohnson/Clipper2 (BSL-1.0). See docs/cpp-optional.md.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=CLIPPER2_SRC");
    println!("cargo:rerun-if-changed=cpp/shim.cpp");
    if std::env::var_os("CARGO_FEATURE_CLIPPER2").is_none() {
        return Ok(());
    }
    let src = PathBuf::from(
        std::env::var_os("CLIPPER2_SRC")
            .ok_or("the clipper2 feature needs CLIPPER2_SRC=<Clipper2 checkout>/CPP/Clipper2Lib (docs/cpp-optional.md)")?,
    );
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(src.join("include"))
        .file("cpp/shim.cpp")
        .file(src.join("src").join("clipper.engine.cpp"))
        .file(src.join("src").join("clipper.offset.cpp"))
        .compile("polyclip_clipper2");
    Ok(())
}
