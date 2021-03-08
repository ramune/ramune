use gl33::*;

use crate::gl::Gl;
use crate::Color;

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
        let vertex_shader_source = r#"
            #version 150
            
            in vec2 aPos;
            in vec2 aTexCoords;
            in vec4 aColor;

            uniform vec2 uViewport;
        
            out vec2 vTexCoords;
            out vec4 vColor;
    
            void main() {
                gl_Position = vec4(aPos.x * 2 / uViewport.x - 1, 1 - aPos.y * 2 / uViewport.y, 0.0, 1.0);
                vTexCoords = aTexCoords;
                vColor = aColor;
            }
        "#;
        let fragment_shader_source = r#"
            #version 150
            
            in vec2 vTexCoords;
            in vec4 vColor;

            uniform sampler2D uSampler;

            out vec4 fragColor;

            void main() {
                fragColor = vColor * texture(uSampler, vTexCoords);
            }
        "#;

        unsafe {
            let vbo = gl.create_vbo();
            let program = gl
                .create_program(vertex_shader_source, fragment_shader_source)
                .expect("Failed to create shader program");

            gl.UseProgram(program);
            gl.BindBuffer(GL_ARRAY_BUFFER, vbo);

            gl.EnableVertexAttribArray(0);
            gl.EnableVertexAttribArray(1);
            gl.EnableVertexAttribArray(2);
            
            #[allow(clippy::erasing_op)]
            gl.VertexAttribPointer(0, 2, GL_FLOAT, 0, 8 * 4, (0 * 4) as *const std::ffi::c_void);
            gl.VertexAttribPointer(1, 2, GL_FLOAT, 0, 8 * 4, (2 * 4) as *const std::ffi::c_void);
            gl.VertexAttribPointer(2, 4, GL_FLOAT, 0, 8 * 4, (4 * 4) as *const std::ffi::c_void);
            // gl.VertexAttribPointer(index, size (# of values), type, stride (bytes), pointer (bytes));

            let mut viewport = [0; 4];
            gl.GetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());
            let viewport_uniform = gl.get_uniform_location(program, "uViewport");
            gl.Uniform2f(viewport_uniform, viewport[2] as _, viewport[3] as _);

            let mut blank_texture = 0;
            gl.GenTextures(1, &mut blank_texture);
            gl.BindTexture(GL_TEXTURE_2D, blank_texture);
            gl.ActiveTexture(GL_TEXTURE0);
            gl.TexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGB as _,
                1,
                1,
                0,
                GL_RGB,
                GL_UNSIGNED_BYTE,
                [255_u8, 255_u8, 255_u8].as_ptr().cast(),
            );

            Self {
                gl,
                vbo,
                program,
                viewport_uniform,
            }
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

    pub fn push(&mut self) -> GraphicsScope<'_> {
        GraphicsScope {
            graphics: self,
            depth: 0.,
            color: Color::WHITE,
            commands: Vec::new(),
        }
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
            if batches.is_empty() {
                batches.push(Batch { vert_count: 0 });
            }
            batches[0].vert_count += command.verts.len();
        }
        self.commands.clear();
        if verts.is_empty() {
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
                0.,
                1.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x,
                y + height,
                0.,
                0.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y,
                1.,
                1.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y,
                1.,
                1.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x,
                y + height,
                0.,
                0.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
                x + width,
                y + height,
                1.,
                0.,
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a,
            ],
            depth: self.depth,
        });
    }
}
