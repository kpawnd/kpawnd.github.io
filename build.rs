fn main() {
    if std::env::var_os("CARGO_FEATURE_CPP_ACCEL").is_some() {
        cc::Build::new()
            .cpp(true)
            .file("cpp/crc32.cpp")
            .flag_if_supported("-O3")
            .compile("cpp_accel");

        println!("cargo:rerun-if-changed=cpp/crc32.cpp");
    }
}
