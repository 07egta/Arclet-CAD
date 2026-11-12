use std::{env, path::PathBuf};
fn main() {
    for key in ["OCCT_INCLUDE_DIR", "OCCT_LIB_DIR"] { println!("cargo:rerun-if-env-changed={key}"); }
    println!("cargo:rerun-if-changed=native/bridge.cpp");
    let inc = env::var_os("OCCT_INCLUDE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/include/opencascade"));
    let lib = env::var_os("OCCT_LIB_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/lib/x86_64-linux-gnu"));
    if !inc.join("Standard_Version.hxx").exists() { panic!("Open CASCADE headers not found. Install OCCT development packages or set OCCT_INCLUDE_DIR and OCCT_LIB_DIR. See README.md."); }
    cc::Build::new().cpp(true).file("native/bridge.cpp").include(&inc).flag_if_supported("-std=c++17").flag_if_supported("/std:c++17").flag_if_supported("/EHsc").warnings(true).compile("arclet_occt_bridge");
    println!("cargo:rustc-link-search=native={}", lib.display());
    let mut libs = vec!["TKOffset", "TKFillet", "TKBool", "TKBO", "TKPrim", "TKMesh", "TKShHealing", "TKTopAlgo", "TKGeomAlgo", "TKBRep", "TKGeomBase", "TKG3d", "TKG2d", "TKMath", "TKernel"];
    if lib.join("libTKDESTEP.so").exists() || lib.join("TKDESTEP.lib").exists() || lib.join("libTKDESTEP.dylib").exists() { libs.extend(["TKDESTEP", "TKDESTL", "TKDE", "TKXSBase"]); }
    else { libs.extend(["TKSTEP", "TKSTEP209", "TKSTEPAttr", "TKSTEPBase", "TKSTL", "TKXSBase"]); }
    for name in libs { println!("cargo:rustc-link-lib=dylib={name}"); }
}
