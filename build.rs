// The binary embeds views/ (the bundled library) and assets/. Cargo tracks the files that existed
// at the last build, not new ones, so a file added to either would be missing from the bundle.
fn main() {
    println!("cargo:rerun-if-changed=views");
    println!("cargo:rerun-if-changed=assets");
}
