fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=HERDSMAN_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=HERDSMAN_BUILD_ID");
    println!("cargo:rerun-if-env-changed=HERDSMAN_BUILD_COMMIT");
}
