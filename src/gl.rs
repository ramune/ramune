use gl33::*;
use std::ops::Deref;
use std::ptr;
use std::rc::Rc;

#[derive(Clone)]
pub struct Gl(Rc<GlFns>);

impl Deref for Gl {
    type Target = GlFns;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Gl {
    pub fn new(gl: GlFns) -> Self {
        Self(Rc::new(gl))
    }

    pub unsafe fn create_shader(&self, kind: u32, source: &str) -> Result<u32, String> {
        if kind != GL_VERTEX_SHADER && kind != GL_FRAGMENT_SHADER {
            return Err("Invalid shader kind!".to_string());
        }

        let shader = self.CreateShader(kind);
        let c_str_vert = std::ffi::CString::new(source).unwrap();
        self.ShaderSource(shader, 1, &c_str_vert.as_ptr(), ptr::null());
        self.CompileShader(shader);

        let mut info_log_length = 0;
        self.GetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut info_log_length);

        let mut info_log_length = 0;
        self.GetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut info_log_length);
        let mut success = GL_FALSE as i32;
        self.GetShaderiv(shader, GL_COMPILE_STATUS, &mut success);
        if success != GL_TRUE as i32 {
            let mut info_log = String::with_capacity(info_log_length as _);
            info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
            let mut length = 0;
            self.GetShaderInfoLog(
                shader,
                info_log_length,
                &mut length,
                (&info_log[..]).as_ptr() as *mut GLchar,
            );
            info_log.truncate(length as _);
            Err(format!(
                "{} shader failed to compile! {}",
                if kind == GL_VERTEX_SHADER {
                    "Vertex"
                } else {
                    "Fragment"
                },
                info_log
            ))
        } else {
            Ok(shader)
        }
    }

    pub unsafe fn create_framebuffer(
        &self,
        width: i32,
        height: i32,
        format: u32,
        kind: u32,
    ) -> (u32, u32) {
        let mut fbo = 0;
        let mut texture = 0;

        self.GenFramebuffers(1, &mut fbo);
        self.BindFramebuffer(GL_FRAMEBUFFER, fbo);

        self.GenTextures(1, &mut texture);
        self.BindTexture(GL_TEXTURE_2D, texture);
        self.TexImage2D(
            GL_TEXTURE_2D,
            0,
            GL_RGB as _,
            width,
            height,
            0,
            format,
            kind,
            ptr::null(),
        );
        self.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as _);
        self.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as _);

        self.FramebufferTexture2D(
            GL_FRAMEBUFFER,
            GL_COLOR_ATTACHMENT0,
            GL_TEXTURE_2D,
            texture,
            0,
        );

        (fbo, texture)
    }

    pub unsafe fn create_program(
        &self,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<u32, String> {
        let vertex_shader = self.create_shader(GL_VERTEX_SHADER, vertex_shader_source)?;
        let fragment_shader = self.create_shader(GL_FRAGMENT_SHADER, fragment_shader_source)?;

        let program = self.CreateProgram();
        self.AttachShader(program, vertex_shader);
        self.AttachShader(program, fragment_shader);
        self.LinkProgram(program);

        let mut info_log_length = 0;
        self.GetProgramiv(program, GL_INFO_LOG_LENGTH, &mut info_log_length);
        let mut success = GL_FALSE as i32;
        self.GetProgramiv(program, GL_LINK_STATUS, &mut success);
        if success != GL_TRUE as i32 {
            let mut info_log = String::with_capacity(info_log_length as _);
            info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
            let mut length = 0;
            self.GetProgramInfoLog(
                program,
                info_log_length,
                &mut length,
                (&info_log[..]).as_ptr() as *mut GLchar,
            );
            info_log.truncate(length as _);
            Err(format!("Shader program failed to link! {}", info_log))
        } else {
            Ok(program)
        }
    }

    pub unsafe fn create_vao(&self) -> u32 {
        let mut vao = 0;
        self.GenVertexArrays(1, &mut vao);
        vao
    }

    pub unsafe fn create_vbo(&self) -> u32 {
        let mut vbo = 0;
        self.GenBuffers(1, &mut vbo);
        vbo
    }

    pub unsafe fn get_uniform_location(&self, program: u32, name: &str) -> i32 {
        let c_str = std::ffi::CString::new(name).unwrap();
        self.GetUniformLocation(program, c_str.as_ptr())
    }
}
