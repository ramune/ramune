use gl33::{GlFns, GL_FRAMEBUFFER_SRGB};
use kapp::{Event as KappEvent, *};
use std::ffi::CStr;
use std::rc::Rc;

mod color;
pub use color::*;
mod event;
pub use event::Event;
mod graphics;
pub use graphics::*;

pub struct Game {
    app: Application,
    event_loop: Option<EventLoop>,
    gl_context: GLContext,
    window: Window,
    graphics: Graphics,
}

pub struct Context {
    _gl: Gl,
}

type Gl = Rc<GlFns>;

impl Game {
    pub fn poll<T: 'static + FnMut(Event)>(mut self, mut callback: T) {
        self.event_loop
            .take()
            .unwrap()
            .run(move |event| match event {
                KappEvent::WindowCloseRequested { .. } => {
                    self.app.quit();
                }
                KappEvent::WindowResized { width, height, .. } => {
                    callback(Event::WindowResized(width, height));
                    self.graphics.set_viewport(width, height);
                }
                KappEvent::Draw { .. } => {
                    callback(Event::Draw(&mut self.graphics));
                    self.gl_context.swap_buffers();
                    self.window.request_redraw();
                }
                _ => {}
            });
    }
}

pub struct GameBuilder {
    title: String,
    size: (u32, u32),
}

impl GameBuilder {
    pub fn new() -> Self {
        Self {
            title: "Ramune".to_string(),
            size: (640, 480),
        }
    }

    pub fn title(&mut self, title: &str) -> &mut Self {
        self.title = title.to_string();
        self
    }

    pub fn size(&mut self, width: u32, height: u32) -> &mut Self {
        self.size = (width, height);
        self
    }

    pub fn build(&self) -> (Game, Context) {
        let (app, event_loop) = initialize();
        let window = app
            .new_window()
            .title(&self.title)
            .size(self.size.0, self.size.1)
            .build()
            .unwrap();
        let mut gl_context = GLContext::new()
            .version_major(3)
            .version_minor(3)
            .build()
            .unwrap();
        gl_context.set_window(Some(&window)).unwrap();

        let gl = unsafe {
            Gl::new(GlFns::load_with(|x| {
                gl_context.get_proc_address(CStr::from_ptr(x).to_str().unwrap()) as *mut _
            }))
        };

        unsafe {
            let mut vao = 0;
            gl.GenVertexArrays(1, &mut vao);
            gl.BindVertexArray(vao);
            gl.Viewport(0, 0, self.size.0 as _, self.size.1 as _); // because kapp uses a dummy window for context creation, it doesn't set the viewport correctly
            gl.Enable(GL_FRAMEBUFFER_SRGB);
        }

        (
            Game {
                app,
                event_loop: Some(event_loop),
                gl_context,
                window,
                graphics: Graphics::new(gl.clone()),
            },
            Context { _gl: gl },
        )
    }
}
