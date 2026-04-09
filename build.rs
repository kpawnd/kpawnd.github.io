fn main() {
    println!("cargo:rustc-check-cfg=cfg(cpp_accel_disabled)");

    if std::env::var_os("CARGO_FEATURE_CPP_ACCEL").is_some() {
        let mut build = cc::Build::new();
        build.cpp(true);
        build.file("cpp/checksum.cpp");
        build.file("cpp/fade.cpp");
        build.file("cpp/raycast.cpp");
        build.file("cpp/collision.cpp");
        build.file("cpp/base64.cpp");
        build.file("cpp/python_core.cpp");
        build.flag_if_supported("-O3");
        build.flag_if_supported("-ffast-math");
        build.flag_if_supported("-fno-rtti");
        build.flag_if_supported("-fno-exceptions");

        if std::env::var_os("CXX").is_none() {
            if cfg!(target_os = "windows") {
                if command_exists("clang++.exe") {
                    build.compiler("clang++.exe");
                } else if command_exists("clang-cl.exe") {
                    build.compiler("clang-cl.exe");
                }
            } else if command_exists("clang++") {
                build.compiler("clang++");
            }
        }

        if let Err(err) = build.try_compile("cpp_accel") {
            println!(
                "cargo:warning=cpp-accel requested but C++ kernel compilation failed ({err}); using Rust fallback"
            );
            println!("cargo:rustc-cfg=cpp_accel_disabled");
        }
    }
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}
