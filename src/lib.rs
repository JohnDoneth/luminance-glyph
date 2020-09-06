//! A fast text renderer for [`luminance`], powered by [`glyph_brush`].
//!
//! Initially forked and modified from [glow_glyph](https://github.com/hecrj/glow_glyph) by [hecrj](https://github.com/hecrj). Many thanks to [hecrj](https://github.com/hecrj)!
//!
//! [`luminance`]: https://github.com/phaazon/luminance-rs
//! [`glyph_brush`]: https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush
#![deny(unused_results)]
mod builder;
mod pipeline;
mod region;

pub use region::Region;

use luminance::{
    backend,
    context::GraphicsContext,
    pipeline::PipelineError,
    pipeline::{Pipeline as LuminancePipeline, TextureBinding},
    pixel::NormR8UI,
    pixel::NormUnsigned,
    shading_gate::ShadingGate,
    tess::Interleaved,
    texture::Dim2,
};

use pipeline::Pipeline;

pub use builder::GlyphBrushBuilder;
pub use glyph_brush::ab_glyph;
pub use glyph_brush::{
    BuiltInLineBreaker, Extra, FontId, GlyphCruncher, GlyphPositioner, HorizontalAlign, Layout,
    LineBreak, LineBreaker, Section, SectionGeometry, SectionGlyph, SectionGlyphIter, SectionText,
    Text, VerticalAlign,
};
pub use pipeline::Instance;

use ab_glyph::{Font, FontArc, Rect};

use core::hash::BuildHasher;
use std::borrow::Cow;

use glyph_brush::{BrushAction, BrushError, DefaultSectionHasher};
use log::{log_enabled, warn};

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
pub struct GlyphBrush<B, F = FontArc, H = DefaultSectionHasher>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: ?Sized
        + backend::texture::Texture<Dim2, NormR8UI>
        + backend::shader::Shader
        + backend::tess::Tess<(), u32, pipeline::Instance, luminance::tess::Interleaved>,
{
    pipeline: Pipeline<B>,
    glyph_brush: glyph_brush::GlyphBrush<Instance, Extra, F, H>,
}

impl<B, F: Font, H: BuildHasher> GlyphBrush<B, F, H>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: backend::texture::Texture<Dim2, NormR8UI>
        + backend::shader::Shader
        + backend::tess::Tess<(), u32, pipeline::Instance, luminance::tess::Interleaved>,
{
    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.queue(section)
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use
    /// [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        G: GlyphPositioner,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.queue_custom_layout(section, custom_layout)
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times.
    #[inline]
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<SectionGlyph>,
        extra: Vec<Extra>,
        bounds: Rect,
    ) {
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds)
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        S: Into<Cow<'a, Section<'a>>>,
        G: GlyphPositioner,
    {
        self.glyph_brush
            .keep_cached_custom_layout(section, custom_layout)
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.keep_cached(section)
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font: F) -> FontId {
        self.glyph_brush.add_font(font)
    }
}

impl<B, F: Font + Sync, H: BuildHasher> GlyphBrush<B, F, H>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: backend::texture::Texture<Dim2, NormR8UI>
        + backend::pipeline::PipelineBase
        + backend::tess::Tess<(), u32, pipeline::Instance, Interleaved>
        + backend::pipeline::PipelineTexture<Dim2, NormR8UI>
        + backend::render_gate::RenderGate
        + backend::tess_gate::TessGate<(), u32, pipeline::Instance, Interleaved>,
{
    /// Draws all queued sections onto a render target.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued<'a>(
        &mut self,
        pipeline: &mut LuminancePipeline<'a, B>,
        shading_gate: &mut ShadingGate<'a, B>,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), PipelineError> {
        self.draw_queued_with_transform(
            pipeline,
            shading_gate,
            orthographic_projection(target_width, target_height),
        )
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued_with_transform<'a>(
        &mut self,
        pipeline: &mut LuminancePipeline<'a, B>,
        shading_gate: &mut ShadingGate<'a, B>,
        transform: [f32; 16],
    ) -> Result<(), PipelineError> {
        //self.process_queued(context);
        self.pipeline.draw(pipeline, shading_gate, transform, None)
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection) and a scissoring region.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued_with_transform_and_scissoring<'a>(
        &mut self,
        pipeline: &mut LuminancePipeline<'a, B>,
        shading_gate: &mut ShadingGate<'a, B>,
        transform: [f32; 16],
        region: Region,
    ) -> Result<(), PipelineError> {
        //self.process_queued(context);
        self.pipeline
            .draw(pipeline, shading_gate, transform, Some(region))
    }

    pub fn process_queued<C>(&mut self, context: &mut C)
    where
        C: GraphicsContext<Backend = B>,
    {
        let pipeline = &mut self.pipeline;

        let mut brush_action;

        loop {
            brush_action = self.glyph_brush.process_queued(
                |rect, tex_data| {
                    let offset = [rect.min[0] as u16, rect.min[1] as u16];
                    let size = [rect.width() as u16, rect.height() as u16];

                    pipeline.update_cache(offset, size, tex_data);
                },
                Instance::from_vertex,
            );

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    // TODO: Obtain max texture dimensions
                    let max_image_dimension = 2048;

                    let (new_width, new_height) = if (suggested.0 > max_image_dimension
                        || suggested.1 > max_image_dimension)
                        && (self.glyph_brush.texture_dimensions().0 < max_image_dimension
                            || self.glyph_brush.texture_dimensions().1 < max_image_dimension)
                    {
                        (max_image_dimension, max_image_dimension)
                    } else {
                        suggested
                    };

                    if log_enabled!(log::Level::Warn) {
                        warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
                            old = self.glyph_brush.texture_dimensions(),
                            new = (new_width, new_height),
                        );
                    }

                    pipeline.increase_cache_size(context, new_width, new_height);
                    self.glyph_brush.resize_texture(new_width, new_height);
                }
            }
        }

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                self.pipeline.upload(context, &verts);
            }
            BrushAction::ReDraw => {}
        };
    }
}

impl<B, F: Font, H: BuildHasher> GlyphBrush<B, F, H>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: ?Sized
        + backend::texture::Texture<Dim2, NormR8UI>
        + backend::shader::Shader
        + backend::tess::Tess<(), u32, pipeline::Instance, luminance::tess::Interleaved>
        + backend::pipeline::PipelineBase
        + backend::pipeline::PipelineTexture<Dim2, NormR8UI>
        + backend::render_gate::RenderGate
        + backend::tess_gate::TessGate<(), u32, pipeline::Instance, Interleaved>,
{
    fn new<C>(context: &mut C, raw_builder: glyph_brush::GlyphBrushBuilder<F, H>) -> Self
    where
        C: GraphicsContext<Backend = B>,
    {
        let glyph_brush = raw_builder.build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();

        GlyphBrush {
            pipeline: Pipeline::new(context, cache_width, cache_height),
            glyph_brush,
        }
    }
}

/// Helper function to generate a generate a transform matrix.
#[rustfmt::skip]
pub fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, -2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}

impl<B, F: Font, H: BuildHasher> GlyphCruncher<F> for GlyphBrush<B, F, H>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: backend::texture::Texture<Dim2, NormR8UI>
        + backend::shader::Shader
        + backend::tess::Tess<(), u32, pipeline::Instance, luminance::tess::Interleaved>,
{
    #[inline]
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyphs_custom_layout(section, custom_layout)
    }

    #[inline]
    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyph_bounds_custom_layout(section, custom_layout)
    }

    #[inline]
    fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }
}

impl<B, F, H> std::fmt::Debug for GlyphBrush<B, F, H>
where
    [[f32; 4]; 4]: backend::shader::Uniformable<B>,
    TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<B>,
    B: backend::texture::Texture<Dim2, NormR8UI>
        + backend::shader::Shader
        + backend::tess::Tess<(), u32, pipeline::Instance, luminance::tess::Interleaved>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlyphBrush")
    }
}
