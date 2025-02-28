fn main() {
    let bindings = bindgen::builder()
        .raw_line("#![allow(warnings)]")
        .header("/usr/include/GLFW/glfw3.h")
        .use_core()
        .generate()
        .unwrap();

    let _gl_bindings = bindgen::builder()
        .raw_line("#![allow(warnings)]")
        .header("/usr/include/GLES3/gl3.h")
        .use_core()
        .generate()
        .unwrap();

    bindings.write_to_file("src/glfw_bindings.rs").unwrap();
    _gl_bindings.write_to_file("src/gl_bindings.rs").unwrap();

    println!("cargo:rustc-link-lib=glfw");
    println!("cargo:rustc-link-lib=GL");
}
