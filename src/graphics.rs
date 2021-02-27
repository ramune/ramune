use core::ptr;
use gl33::*;

use crate::{Color, Gl};

pub struct Graphics {
    gl: Gl,
    vbo: u32,
    fbo_composite: u32,
    fbo_intermediary: u32,
    texture_composite: u32,
    texture_intermediary: u32,
    program_composite: u32,
    program_swapchain: u32,
    viewport_size: (u32, u32),
    viewport_uniform: i32,
}

impl Drop for Graphics {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteBuffers(1, &self.vbo);
            self.gl.DeleteProgram(self.program_composite);
        }
    }
}

impl Graphics {
    pub fn new(gl: Gl) -> Self {
        let vertex_render = r#"
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

        let fragment_render = r#"
            #version 150
            
            in vec4 vColor;

            out vec4 fragColor;

            void main() {
                fragColor = vColor;
            }
        "#;

        let vertex_swapchain = r#"
            #version 150

            out vec2 xy;
            
            void main() {
                xy = vec2(float(-1 + int(gl_VertexID > 1) * 2), float(-1 + int(gl_VertexID % 2) * 2));
                gl_Position = vec4(xy, 0.0, 0.0);
            }
        "#;

        let fragment_swapchain = r#"
            #version 150
            
            in vec2 xy;

            uniform sampler2D uSampler;

            out vec4 fragColor;

            void main() {
                fragColor = texture(uSampler, (xy + 1.0) * 0.5);
            }
        "#;

        unsafe {
            let mut vbo = 0;
            gl.GenBuffers(1, &mut vbo);

            let program_composite = create_program(&gl, vertex_render, fragment_render)
                .expect("Unable to create shader program");
            let program_swapchain = create_program(&gl, vertex_swapchain, fragment_swapchain)
                .expect("Unable to create shader program");

            gl.BindBuffer(GL_ARRAY_BUFFER, vbo);
            gl.EnableVertexAttribArray(0);
            gl.EnableVertexAttribArray(1);
            // glVertexAttribPointer(index, size (# of values), type, stride (bytes), pointer (bytes))
            gl.VertexAttribPointer(0, 2, GL_FLOAT, 0, 6 * 4, 0 as *const std::ffi::c_void);
            gl.VertexAttribPointer(1, 4, GL_FLOAT, 0, 6 * 4, (2 * 4) as *const std::ffi::c_void);

            let mut viewport = [0; 4];
            gl.GetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());

            let viewport_size = (viewport[2] as _, viewport[3] as _);

            let (fbo_composite, texture_composite) =
                create_framebuffer(&gl, viewport[2], viewport[3], GL_RGB, GL_HALF_FLOAT);
            let (fbo_intermediary, texture_intermediary) =
                create_framebuffer(&gl, viewport[2], viewport[3], GL_SRGB, GL_UNSIGNED_BYTE);

            gl.UseProgram(program_composite);
            let c_str = std::ffi::CString::new("uViewport").unwrap();
            let viewport_uniform = gl.GetUniformLocation(program_composite, c_str.as_ptr());
            gl.Uniform2f(viewport_uniform, viewport[2] as _, viewport[3] as _);

            Self {
                gl,
                vbo,
                fbo_composite,
                fbo_intermediary,
                texture_composite,
                texture_intermediary,
                program_composite,
                program_swapchain,
                viewport_size,
                viewport_uniform,
            }
        }
    }

    pub(crate) fn set_viewport(&mut self, width: u32, height: u32) {
        unsafe {
            self.gl.Viewport(0, 0, width as _, height as _);
            self.gl
                .Uniform2f(self.viewport_uniform, width as _, height as _);
            self.viewport_size = (width, height);
        }
    }

    pub fn clear(&mut self, color: Color) {
        unsafe {
            self.gl.BindFramebuffer(GL_FRAMEBUFFER, self.fbo_composite);
            self.gl.ClearColor(color.r, color.g, color.b, color.a);
            self.gl.Clear(GL_COLOR_BUFFER_BIT);
            self.gl.BindFramebuffer(GL_FRAMEBUFFER, self.fbo_intermediary);
            self.gl.Clear(GL_COLOR_BUFFER_BIT);
        }
    }

    pub fn push(&mut self) -> GraphicsScope<'_> {
        GraphicsScope {
            graphics: self,
            depth: 0.,
            color: Color::WHITE,
            commands: Vec::new(),
        }
    }
}

unsafe fn create_shader(gl: &Gl, kind: u32, source: &str) -> Result<u32, String> {
    if kind != GL_VERTEX_SHADER && kind != GL_FRAGMENT_SHADER {
        return Err("Invalid shader kind!".to_string());
    }

    let shader = gl.CreateShader(kind);
    let c_str_vert = std::ffi::CString::new(source).unwrap();
    gl.ShaderSource(shader, 1, &c_str_vert.as_ptr(), ptr::null());
    gl.CompileShader(shader);

    let mut info_log_length = 0;
    gl.GetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut info_log_length);

    let mut info_log_length = 0;
    gl.GetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut info_log_length);
    let mut success = GL_FALSE as i32;
    gl.GetShaderiv(shader, GL_COMPILE_STATUS, &mut success);
    if success != GL_TRUE as i32 {
        let mut info_log = String::with_capacity(info_log_length as _);
        info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
        let mut length = 0;
        gl.GetShaderInfoLog(
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

unsafe fn create_framebuffer(
    gl: &Gl,
    width: i32,
    height: i32,
    format: u32,
    kind: u32,
) -> (u32, u32) {
    let mut fbo = 0;
    let mut texture = 0;

    gl.GenFramebuffers(1, &mut fbo);
    gl.BindFramebuffer(GL_FRAMEBUFFER, fbo);

    gl.GenTextures(1, &mut texture);
    gl.BindTexture(GL_TEXTURE_2D, texture);
    gl.TexImage2D(
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
    gl.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as _);
    gl.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as _);

    gl.FramebufferTexture2D(
        GL_FRAMEBUFFER,
        GL_COLOR_ATTACHMENT0,
        GL_TEXTURE_2D,
        texture,
        0,
    );

    (fbo, texture)
}

unsafe fn create_program(
    gl: &Gl,
    vertex_shader_source: &str,
    fragment_shader_source: &str,
) -> Result<u32, String> {
    let vertex_shader = create_shader(gl, GL_VERTEX_SHADER, vertex_shader_source)?;
    let fragment_shader = create_shader(gl, GL_FRAGMENT_SHADER, fragment_shader_source)?;

    let program = gl.CreateProgram();
    gl.AttachShader(program, vertex_shader);
    gl.AttachShader(program, fragment_shader);
    gl.LinkProgram(program);

    let mut info_log_length = 0;
    gl.GetProgramiv(program, GL_INFO_LOG_LENGTH, &mut info_log_length);
    let mut success = GL_FALSE as i32;
    gl.GetProgramiv(program, GL_LINK_STATUS, &mut success);
    if success != GL_TRUE as i32 {
        let mut info_log = String::with_capacity(info_log_length as _);
        info_log.extend(std::iter::repeat('\0').take(info_log_length as _));
        let mut length = 0;
        gl.GetProgramInfoLog(
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

pub struct GraphicsScope<'a> {
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

impl Drop for GraphicsScope<'_> {
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
            let gl = &self.graphics.gl;

            gl.BufferData(
                GL_ARRAY_BUFFER,
                (std::mem::size_of::<f32>() * verts.len()) as isize,
                verts.as_ptr() as *const _ as *const std::ffi::c_void,
                GL_STREAM_DRAW,
            );

            gl.BindFramebuffer(GL_DRAW_FRAMEBUFFER, self.graphics.fbo_composite);
            gl.UseProgram(self.graphics.program_composite);
            let mut index = 0;
            for batch in batches {
                self.graphics.gl.DrawArrays(
                    GL_TRIANGLES,
                    index as _,
                    (index + batch.vert_count) as _,
                );
                index += batch.vert_count;
            }
            gl.BindFramebuffer(GL_FRAMEBUFFER, 0);
            gl.BindFramebuffer(GL_READ_FRAMEBUFFER, self.graphics.fbo_composite);
            gl.BindFramebuffer(GL_DRAW_FRAMEBUFFER, self.graphics.fbo_intermediary);
            gl.UseProgram(self.graphics.program_swapchain);
            // let (w, h) = self.graphics.viewport_size;
            let c_str = std::ffi::CString::new("uSampler").unwrap();
            let tex_uniform = gl.GetUniformLocation(self.graphics.program_swapchain, c_str.as_ptr());
            gl.BindTexture(GL_TEXTURE0, self.graphics.texture_composite);
            gl.BindTexture(GL_TEXTURE1, self.graphics.texture_intermediary);
            gl.Uniform1i(tex_uniform, 0);
            gl.DrawArrays(GL_TRIANGLE_STRIP, 0, 4);
            // gl.BlitFramebuffer(
            //     0,
            //     0,
            //     w as _,
            //     h as _,
            //     0,
            //     0,
            //     w as _,
            //     h as _,
            //     GL_COLOR_BUFFER_BIT,
            //     GL_NEAREST,
            // );
            gl.BindFramebuffer(GL_DRAW_FRAMEBUFFER, 0);
            gl.Uniform1i(tex_uniform, 1);
            gl.DrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        }
    }
}

impl GraphicsScope<'_> {
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
