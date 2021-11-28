use glfw::{Action, Context as _, Key, SwapInterval, WindowEvent, WindowMode};
use glyph_brush::Text;
use luminance::{context::GraphicsContext as _, pipeline::PipelineState};
use luminance_glfw::{GlfwSurface, GlfwSurfaceError};
use luminance_glyph::{ab_glyph, GlyphBrushBuilder, Section};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let surface = GlfwSurface::new(|glfw| {
        let (mut window, events) = glfw
            .create_window(1024, 720, "Luminance Glyph", WindowMode::Windowed)
            .ok_or(GlfwSurfaceError::UserError(()))?;

        window.make_current();
        window.set_all_polling(true);
        glfw.set_swap_interval(SwapInterval::Sync(1));

        Ok((window, events))
    })
    .expect("GLFW surface creation");

    let mut context = surface.context;

    let mut glyph_brush = GlyphBrushBuilder::using_font(ab_glyph::FontArc::try_from_slice(
        include_bytes!("Inconsolata-Regular.ttf"),
    )?)
    .build(&mut context);

    let mut resize = false;
    let mut back_buffer = context.back_buffer().unwrap();

    'app: loop {
        context.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }

                // Handle window resizing.
                WindowEvent::FramebufferSize(_width, _height) => {
                    resize = true;
                }

                _ => {}
            }
        }

        if resize {
            // Simply ask another backbuffer at the right dimension (no allocation / reallocation).
            back_buffer = context.back_buffer().unwrap();
            context.back_buffer().unwrap();
            resize = false;
        }

        let (width, height) = context.window.get_size();
        glyph_brush.queue(Section {
            screen_position: (30.0, 30.0),
            bounds: (width as f32, height as f32),
            text: vec![Text::default()
                .with_text("Hello luminance_glyph!")
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(40.0)],
            ..Section::default()
        });

        glyph_brush.process_queued(&mut context);

        let render = context.new_pipeline_gate().pipeline(
            &back_buffer,
            &PipelineState::default().set_clear_color([0.2, 0.2, 0.2, 1.0]),
            |mut pipeline, mut shd_gate| {
                glyph_brush.draw_queued(&mut pipeline, &mut shd_gate, 1024, 720)
            },
        );

        render.assume().into_result()?;
        context.window.swap_buffers();
    }

    Ok(())
}
