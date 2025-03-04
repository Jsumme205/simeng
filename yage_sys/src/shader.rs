use small_vec::SmallVec;

use crate::{gl_bindings::glUseProgram, window::RawWindow};
use core::{convert::AsRef, marker::PhantomData};

use crate::gl_bindings::{
    glBindBuffer, glBindVertexArray, glBufferData, glCompileShader, glCreateShader,
    glEnableVertexAttribArray, glGenBuffers, glGenVertexArrays, glShaderSource,
    glVertexAttribPointer, GL_ARRAY_BUFFER, GL_FALSE, GL_FLOAT, GL_FRAGMENT_SHADER, GL_STATIC_DRAW,
    GL_VERTEX_SHADER,
};

mod sealed {
    pub trait Sealed {}
}

pub trait ProgramMarker: sealed::Sealed {
    fn kind() -> u32;
}

pub enum VertexShader {}
pub enum FragmentShader {}

impl sealed::Sealed for VertexShader {}
impl sealed::Sealed for FragmentShader {}

impl ProgramMarker for VertexShader {
    fn kind() -> u32 {
        GL_VERTEX_SHADER
    }
}
impl ProgramMarker for FragmentShader {
    fn kind() -> u32 {
        GL_FRAGMENT_SHADER
    }
}

macro_rules!  impl_vector_count{
    ($($n:literal)*) => {
        $(
            impl sealed::Sealed for VectorCount<{$n}> {}
            impl SupportedVectorCount for VectorCount<{$n}> {}
        )*
    };
}

impl_vector_count!(2 3 4);

pub trait SupportedVectorCount: sealed::Sealed {}

pub enum VectorCount<const N: usize> {}

struct Atrributes<const N: usize> {
    inner: SmallVec<[f32; N]>,
}

impl<const N: usize> Atrributes<N> {
    pub const fn new() -> Self {
        Self {
            inner: SmallVec::new(),
        }
    }

    fn push_attribute_line(&mut self, attrib_line: [f32; N]) {
        self.inner.push(attrib_line);
    }

    fn len(&self) -> usize {
        self.inner.len() * 8
    }

    fn as_mut_ptr(&mut self) -> *mut f32 {
        self.inner.as_mut_ptr().cast()
    }

    unsafe fn __build(mut self) -> BufferObjects {
        let mut shaders = BufferObjects { vao: 0, vbo: 0 };

        glGenVertexArrays(1, &raw mut shaders.vao);
        glGenBuffers(1, &raw mut shaders.vbo);
        glBindVertexArray(shaders.vao);
        glBindBuffer(GL_ARRAY_BUFFER, shaders.vbo);
        glBufferData(
            GL_ARRAY_BUFFER,
            self.len() as _,
            self.as_mut_ptr() as *mut _,
            GL_STATIC_DRAW,
        );

        use crate::gl_bindings::GLsizei;

        glVertexAttribPointer(
            0,
            3,
            GL_FLOAT,
            GL_FALSE as _,
            6 * core::mem::size_of::<f32>() as GLsizei,
            0 as *const _,
        );
        glEnableVertexAttribArray(0);

        glVertexAttribPointer(
            1,
            3,
            GL_FLOAT,
            GL_FALSE as _,
            6 * core::mem::size_of::<f32>() as GLsizei,
            (3 * core::mem::size_of::<f32>()) as *const _,
        );
        glEnableVertexAttribArray(1);

        shaders
    }
}

pub struct ShaderLoader<const N: usize, K: ProgramMarker> {
    source: SmallVec<u8>,
    attributes: Atrributes<N>,
    _ph: PhantomData<K>,
}

impl<const N: usize, K> ShaderLoader<N, K>
where
    K: ProgramMarker,
{
    #[cfg(feature = "std")]
    pub fn load<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        use std::io::Read;

        let mut file = std::fs::File::open(path)?;
        let size = file.metadata()?.len();
        let mut small = SmallVec::with_capacity(size as _);
        file.read_exact(&mut small)?;
        Ok(Self {
            source: small,
            attributes: Atrributes::new(),
            _ph: PhantomData,
        })
    }

    pub fn from_raw_source(bytes: SmallVec<u8>) -> Self {
        Self {
            source: bytes,
            attributes: Atrributes::new(),
            _ph: PhantomData,
        }
    }

    pub fn push_attr_line(&mut self, attrib_line: [f32; N]) -> &mut Self {
        self.attributes.push_attribute_line(attrib_line);
        self
    }

    pub unsafe fn compile(self) -> CompiledShaders<K> {
        let id = glCreateShader(K::kind());
        let ptr = self.source.as_ptr() as *const i8;
        glShaderSource(id, 1, &raw const ptr, core::ptr::null());
        glCompileShader(id);
        CompiledShaders {
            id,
            buffer: self.attributes.__build(),
            _marker1: PhantomData,
        }
    }
}

pub struct CompiledShaders<K: ProgramMarker> {
    buffer: BufferObjects,
    id: u32,
    _marker1: PhantomData<K>,
}

impl<K: ProgramMarker> CompiledShaders<K> {
    pub fn use_shader(&mut self, _: &RawWindow) {
        unsafe {
            glUseProgram(self.id);
        }
    }
}

struct BufferObjects {
    vao: u32,
    vbo: u32,
}
