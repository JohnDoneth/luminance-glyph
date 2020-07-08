
# luminance-glyph

[![Integration status](https://github.com/JohnDoneth/luminance-glyph/workflows/Integration/badge.svg)](https://github.com/JohnDoneth/luminance-glyph/actions)
[![crates.io](https://img.shields.io/crates/v/luminance-glyph.svg)](https://crates.io/crates/luminance-glyph)
[![Documentation](https://docs.rs/luminance-glyph/badge.svg)](https://docs.rs/luminance-glyph)
[![License](https://img.shields.io/crates/l/luminance-glyph.svg)](https://github.com/JohnDoneth/luminance-glyph/blob/master/LICENSE)

A fast text renderer for [luminance](https://github.com/phaazon/luminance-rs), powered by [glyph_brush](https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush). Initially forked and modified from [glow_glyph](https://github.com/hecrj/glow_glyph) by [hecrj](https://github.com/hecrj). Many thanks to [hecrj](https://github.com/hecrj)!

```rust
let mut glyph_brush = GlyphBrushBuilder::using_font(ab_glyph::FontArc::try_from_slice(
        include_bytes!("Inconsolata-Regular.ttf"),
    )?)
    .build(&mut surface);
    
glyph_brush.queue(
    Section::default().add_text(
        Text::new("Hello Luminance Glyph")
            .with_color([1.0, 1.0, 1.0, 1.0])
            .with_scale(80.0),
    ),
);

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
```
