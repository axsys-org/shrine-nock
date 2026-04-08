use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let is_macos = std::env::consts::OS == "macos";
    // Get the directory containing the Cargo.toml (manifest dir)
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vere_src_dir = manifest_dir.join("vere");
    let vere_out_dir = vere_src_dir.join("zig-out");
    let lib_dir = vere_out_dir.join("lib");
    let include_dir = vere_out_dir.join("include");

    let noun_header = include_dir.join("noun.h");

    // Set library search path (using absolute path)
    if is_macos {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        let darwin_dir = PathBuf::from("/opt/homebrew/opt/llvm@18/lib/clang/18/lib/darwin");
        println!("cargo:rustc-link-search=native={}", darwin_dir.display());
        // Embed rpath so the dynamic UBSan library can be found at runtime
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", darwin_dir.display());
    } else if std::env::consts::OS == "linux" {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        let clang_dir = PathBuf::from("/usr/lib/clang/19/lib/linux");
        let arch = std::env::consts::ARCH;
        let linux_dir = match arch {
            "aarch64" => PathBuf::from("/usr/lib/aarch64-linux-gnu"),
            "x86_64" => PathBuf::from("/usr/lib/x86_64-linux-gnu"),
            _ => panic!("Unsupported architecture: {}", arch),
        };
        // println!("cargo:rustc-link-search=native={}", linux_dir.display());
        println!("cargo:rustc-link-search=native={}", clang_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", linux_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", clang_dir.display());
    } else {
        panic!("Unsupported OS: {}", std::env::consts::OS);
    }

    // Set libraries to link (order matters for static linking)
    // vere includes noun and all its dependencies
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=ssl");
    println!("cargo:rustc-link-lib=static=secp256k1");
    println!("cargo:rustc-link-lib=static=aes_siv");
    println!("cargo:rustc-link-lib=static=softfloat");
    println!("cargo:rustc-link-lib=static=blake3");
    println!("cargo:rustc-link-lib=static=keccak_tiny");
    println!("cargo:rustc-link-lib=static=scrypt");
    println!("cargo:rustc-link-lib=static=argon2");
    println!("cargo:rustc-link-lib=static=pdjson");
    println!("cargo:rustc-link-lib=static=murmur3");
    println!("cargo:rustc-link-lib=static=backtrace");
    println!("cargo:rustc-link-lib=static=ed25519");
    println!("cargo:rustc-link-lib=static=ge_additions");
    println!("cargo:rustc-link-lib=static=c3");
    println!("cargo:rustc-link-lib=static=gmp");
    println!("cargo:rustc-link-lib=static=urcrypt");
    println!("cargo:rustc-link-lib=static=ent");
    println!("cargo:rustc-link-lib=static=ur");
    println!("cargo:rustc-link-lib=static=wasm3");
    println!("cargo:rustc-link-lib=static=whereami");

    println!("cargo:rustc-link-lib=static=monocypher");
    println!("cargo:rustc-link-lib=static=softblas");
    println!("cargo:rustc-link-lib=static=sigsegv");
    if is_macos {
        // println!("cargo:rustc-link-lib=clang_rt.ubsan_osx_dynamic");
        // println!("cargo:rustc-link-lib=clang_rt.asan_osx_dynamic");
    } else if std::env::consts::OS == "linux" {
        // println!("cargo:rustc-link-lib=clang_rt.ubsan_linux_dynamic");
    } else {
        panic!("Unsupported OS: {}", std::env::consts::OS);
    }

    println!("cargo:rustc-link-lib=static=noun");
    // Regenerate bindings if header changes
    // println!("cargo:rerun-if-changed=vere");

    let mut zig = Command::new("zig");
    zig.arg("build");
    zig.arg("-Drelease");
    // zig.arg("-Dmem-dbg");
    // zig.arg("-Dubsan");
    // zig.arg("-Dasan");
    zig.current_dir(&vere_src_dir);
    zig.output().expect("Failed to build vere");

    let include_arg = format!("-I{}", include_dir.display());

    let os_define = match std::env::consts::OS {
        "macos" => "-DU3_OS_osx",
        "linux" => "-DU3_OS_linux",
        "windows" => "-DU3_OS_windows",
        _ => panic!("Unsupported OS: {}", std::env::consts::OS),
    };

    let bindings = bindgen::Builder::default()
        .clang_arg("-fno-sanitize=all")
        .header(noun_header.to_str().unwrap())
        .clang_arg(&include_arg)
        .clang_arg(os_define)
        .clang_arg("-DU3_OS_ENDIAN_little")
        .wrap_unsafe_ops(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
