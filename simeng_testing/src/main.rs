use std::ffi::CString;

use simeng_sys::{
    shader::{FragmentShader, ShaderLoader, VertexShader},
    window::RawWindowParams,
};

fn main() {
    simeng_sys::glfw_init().unwrap();
    simeng_sys::register_error_callback(simeng_sys::DefaultErrorCallback);

    /*
    let mut vertex_shader: ShaderLoader<6, VertexShader> =
        ShaderLoader::load("vertex-shader.vs").unwrap();
    vertex_shader
        .push_attr_line([0.5, -0.5, 0.0, 1.0, 0.0, 0.0])
        .push_attr_line([-0.5, -0.5, 0.0, 0.0, 1.0, 0.0])
        .push_attr_line([0.0, 0.5, 0.0, 0.0, 0.0, 1.0]);
    let mut v_compiled = unsafe { vertex_shader.compile() };

    let mut frag_shader: ShaderLoader<6, FragmentShader> =
        ShaderLoader::load("frag-shader.fs").unwrap();
    frag_shader
        .push_attr_line([0.5, -0.5, 0.0, 1.0, 0.0, 0.0])
        .push_attr_line([-0.5, -0.5, 0.0, 0.0, 1.0, 0.0])
        .push_attr_line([0.0, 0.5, 0.0, 0.0, 0.0, 1.0]);
    let mut f_compiled = unsafe { frag_shader.compile() };

    */

    let mut window = simeng_sys::window::RawWindow::create(RawWindowParams {
        width: 200,
        height: 200,
        name: Some(CString::new("test").unwrap()),
        key_handler: None,
    })
    .unwrap();

    //v_compiled.use_shader(&window);
    //f_compiled.use_shader(&window);

    let _ = unsafe { window.main_loop(|_| Ok(())) };

    println!("Hello, world!");
}
