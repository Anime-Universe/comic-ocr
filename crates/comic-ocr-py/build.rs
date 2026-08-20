fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    if std::env::var("CARGO_CFG_TARGET_OS")
        .map(|s| s == "macos")
        .unwrap_or(false)
    {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
