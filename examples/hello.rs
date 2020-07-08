use glyph_brush::Text;
use luminance::context::GraphicsContext as _;
use luminance::pipeline::PipelineState;
use luminance_glfw::{Action, GlfwSurface, Key, Surface, WindowDim, WindowEvent, WindowOpt};
use luminance_glyph::{ab_glyph, GlyphBrushBuilder, Section};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(1024, 720),
        "Luminance Glyph",
        WindowOpt::default().set_num_samples(2),
    )
    .expect("GLFW surface creation");

    let mut glyph_brush = GlyphBrushBuilder::using_font(ab_glyph::FontArc::try_from_slice(
        include_bytes!("Inconsolata-Regular.ttf"),
    )?)
    .build(&mut surface);

    let mut resize = false;
    let mut back_buffer = surface.back_buffer().unwrap();

    'app: loop {
        for event in surface.poll_events() {
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
            back_buffer = surface.back_buffer().unwrap();
            surface.back_buffer().unwrap();
            resize = false;
        }

        glyph_brush.queue(Section {
            screen_position: (30.0, 30.0),
            bounds: (surface.width() as f32, surface.height() as f32),
            text: vec![Text::default()
                .with_text("Hello luminance_glyph!")
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(40.0)],
            ..Section::default()
        });

        glyph_brush.process_queued(&mut surface);

        surface.pipeline_builder().pipeline(
            &back_buffer,
            &PipelineState::default().set_clear_color([0.2, 0.2, 0.2, 1.0]),
            |mut pipeline, mut shd_gate| {
                glyph_brush
                    .draw_queued(&mut pipeline, &mut shd_gate, 1024, 720)
                    .expect("failed to render glyphs");
            },
        );

        surface.swap_buffers();
    }

    Ok(())
}
