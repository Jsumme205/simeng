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

    //let _ = unsafe { window.main_loop(|_| Ok(())) };

    use simeng_task::builder::Builder;

    let (sender, recv) = flume::unbounded();

    let schedule = move |task| sender.send(task).unwrap();

    let (task, handle) =
        simeng_task::builder::Builder::new().spawn(move |()| async move { 1 + 2 }, schedule);
    dbg!(task.state());
    task.schedule();

    let task = recv.recv().unwrap();

    dbg!(task.state());

    let waker = task.waker();
    let mut cx = core::task::Context::from_waker(&waker);
    task.run(&mut cx);

    //println!("{:?}", handle.join());

    println!("Hello, world!");
}
