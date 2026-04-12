fn main() {
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=DAMSEL_TARGET_TRIPLE={target}");
    }
}
