use gl33::*;

use crate::{Color, Gl};

pub struct Graphics {
    gl: Gl,
    vbo: u32,
    program: u32,
    viewport_uniform: i32,
}

impl Drop for Graphics {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteBuffers(1, &self.vbo);
            self.gl.DeleteProgram(self.program);
        }
    }
}

impl Graphics {
    pub fn new(gl: Gl) -> Self {
        let mut vbo = 0;
        let program;
        let viewport_uniform;

        let vertex_shader_source = r#"
            #version 150
            
            in vec2 aPos;
            in vec4 aColor;

            uniform vec2 uViewport;
    
            out vec4 vColor;
    
            void main() {
                gl_Position = vec4(aPos.x * 2 / uViewport.x - 1, 1 - aPos.y * 2 / uViewport.y, 0.0, 1.0);
                vColor = aColor;
            }
        "#;
        let fragment_shader_source = r#"
            #version 150
            
            in vec4 vColor;

            out vec4 fragColor;

            void main() {
                fragColor = vColor;
            }
        "#;

        unsafe {
            gl.GenBuffers(1, &mut vbo);

            let vertex_shader = gl.CreateShader(GL_VERTEX_SHADER);
            let c_str_vert = std::ffi::CString::new(vertex_shader_source).unwrap();
            gl.ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), std::ptr::null());
            gl.CompileShader(vertex_shader);

            let mut info_log_length = 0;
            gl.GetShaderiv(vertex_shader, GL_INFO_LOG_LENGTH, &mut info_log_length);

            let mut success = GL_FALSE as i32;
            gl.GetShaderiv(vertex_shader, GL_COMPILE_STATUS, &mut success);
            if success != GL_TRUE as i32 {
                let mut info_log = String::with_capacity(info_log_length as _);
                info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
                let mut length = 0;
                gl.GetShaderInfoLog(
                    vertex_shader,
                    info_log_length,
                    &mut length,
                    (&info_log[..]).as_ptr() as *mut GLchar,
                );
                info_log.truncate(length as _);
                eprintln!("Vertex shader failed to compile!\n{}", info_log,);
            }

            let fragment_shader = gl.CreateShader(GL_FRAGMENT_SHADER);
            let c_str_frag = std::ffi::CString::new(fragment_shader_source).unwrap();
            gl.ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), std::ptr::null());
            gl.CompileShader(fragment_shader);

            let mut success = GL_FALSE as i32;
            gl.GetShaderiv(fragment_shader, GL_COMPILE_STATUS, &mut success);
            if success != GL_TRUE as i32 {
                let mut info_log = String::with_capacity(info_log_length as _);
                info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
                let mut length = 0;
                gl.GetShaderInfoLog(
                    fragment_shader,
                    info_log_length,
                    &mut length,
                    (&info_log[..]).as_ptr() as *mut GLchar,
                );
                info_log.truncate(length as _);
                eprintln!("Fragment shader failed to compile!\n{}", info_log,);
            }

            program = gl.CreateProgram();
            gl.AttachShader(program, vertex_shader);
            gl.AttachShader(program, fragment_shader);
            gl.LinkProgram(program);

            let mut success = GL_FALSE as i32;
            gl.GetProgramiv(program, GL_LINK_STATUS, &mut success);
            if success != GL_TRUE as i32 {
                let mut info_log = String::with_capacity(info_log_length as _);
                info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
                let mut length = 0;
                gl.GetProgramInfoLog(
                    fragment_shader,
                    info_log_length,
                    &mut length,
                    (&info_log[..]).as_ptr() as *mut GLchar,
                );
                info_log.truncate(length as _);
                eprintln!("Shader program failed to link!\n{}", info_log,);
            }

            gl.UseProgram(program);
            gl.BindBuffer(GL_ARRAY_BUFFER, vbo);
            gl.EnableVertexAttribArray(0);
            gl.EnableVertexAttribArray(1);
            // glVertexAttribPointer(index, size (# of values), type, stride (bytes), pointer (bytes))
            gl.VertexAttribPointer(0, 2, GL_FLOAT, 0, 6 * 4, 0 as *const std::ffi::c_void);
            gl.VertexAttribPointer(1, 4, GL_FLOAT, 0, 6 * 4, (2 * 4) as *const std::ffi::c_void);

            let c_str = std::ffi::CString::new("uViewport").unwrap();
            viewport_uniform = gl.GetUniformLocation(program, c_str.as_ptr());
            let mut viewport = [0; 4];
            gl.GetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());
            gl.Uniform2f(viewport_uniform, viewport[2] as _, viewport[3] as _);
        }

        Self {
            gl,
            vbo,
            program,
            viewport_uniform,
        }
    }

    pub(crate) fn set_viewport(&mut self, width: u32, height: u32) {
        unsafe {
            self.gl.Viewport(0, 0, width as _, height as _);
            self.gl
                .Uniform2f(self.viewport_uniform, width as _, height as _);
        }
    }

    pub fn clear(&mut self, color: Color) {
        unsafe {
            self.gl.ClearColor(color.r, color.g, color.b, color.a);
            self.gl.Clear(GL_COLOR_BUFFER_BIT);
        }
    }

    pub fn push(&mut self) -> Painter<'_> {
        Painter {
            graphics: self,
            depth: 0.,
            color: Color::WHITE,
            commands: Vec::new(),
        }
    }
}

pub struct Painter<'a> {
    graphics: &'a mut Graphics,
    depth: f32,
    color: Color,
    commands: Vec<Command>,
}

struct Command {
    verts: Vec<f32>,
    depth: f32,
}

struct Batch {
    vert_count: usize,
}

impl Drop for Painter<'_> {
    fn drop(&mut self) {
        let mut batches = Vec::new();
        let mut verts = Vec::<f32>::new();
        self.commands
            .sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());
        for command in self.commands.iter() {
            verts.extend(command.verts.iter());
            if batches.len() == 0 {
                batches.push(Batch { vert_count: 0 });
            }
            batches[0].vert_count += command.verts.len();
        }
        self.commands.clear();
        if verts.len() == 0 {
            return;
        }

        unsafe {
            self.graphics.gl.BufferData(
                GL_ARRAY_BUFFER,
                (std::mem::size_of::<f32>() * verts.len()) as isize,
                verts.as_ptr() as *const _ as *const std::ffi::c_void,
                GL_STREAM_DRAW,
            );

            let mut index = 0;
            for batch in batches {
                self.graphics.gl.DrawArrays(
                    GL_TRIANGLES,
                    index as _,
                    (index + batch.vert_count) as _,
                );
                index += batch.vert_count;
            }
        }
    }
}

impl Painter<'_> {
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.commands.push(Command {
            verts: vec![
                x,
                y,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x,
                y + height,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x,
                y + height,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y + height,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
            ],
            depth: self.depth,
        });
    }
}
