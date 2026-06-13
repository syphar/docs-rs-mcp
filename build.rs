fn main() {
    // Expose the target triple this binary was compiled for, so the server
    // can default to fetching docs.rs builds for the developer's host.
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
