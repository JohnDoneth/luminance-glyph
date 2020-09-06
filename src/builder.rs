use super::GlyphBrush;
use crate::Instance;
use core::hash::BuildHasher;
use glyph_brush::{ab_glyph::Font, delegate_glyph_brush_builder_fns, DefaultSectionHasher};
use luminance::{
    backend, context::GraphicsContext, pipeline::TextureBinding, pixel::NormR8UI,
    pixel::NormUnsigned, tess::Interleaved, texture::Dim2,
};

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
pub struct GlyphBrushBuilder<F, H = DefaultSectionHasher> {
    inner: glyph_brush::GlyphBrushBuilder<F, H>,
}

impl<F, H> From<glyph_brush::GlyphBrushBuilder<F, H>> for GlyphBrushBuilder<F, H> {
    fn from(inner: glyph_brush::GlyphBrushBuilder<F, H>) -> Self {
        GlyphBrushBuilder { inner }
    }
}

impl GlyphBrushBuilder<()> {
    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font<F: Font>(font: F) -> GlyphBrushBuilder<F> {
        Self::using_fonts(vec![font])
    }

    pub fn using_fonts<F: Font>(fonts: Vec<F>) -> GlyphBrushBuilder<F> {
        GlyphBrushBuilder {
            inner: glyph_brush::GlyphBrushBuilder::using_fonts(fonts),
        }
    }
}

impl<F: Font, H: BuildHasher> GlyphBrushBuilder<F, H> {
    delegate_glyph_brush_builder_fns!(inner);

    /// When multiple CPU cores are available spread rasterization work across
    /// all cores.
    ///
    /// Significantly reduces worst case latency in multicore environments.
    ///
    /// # Platform-specific behaviour
    ///
    /// This option has no effect on wasm32.
    pub fn draw_cache_multithread(mut self, multithread: bool) -> Self {
        self.inner.draw_cache_builder = self.inner.draw_cache_builder.multithread(multithread);

        self
    }

    /// Sets the section hasher. `GlyphBrush` cannot handle absolute section
    /// hash collisions so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap
    /// internal use.
    ///
    /// Defaults to [seahash](https://docs.rs/seahash).
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<F, T> {
        GlyphBrushBuilder {
            inner: self.inner.section_hasher(section_hasher),
        }
    }

    /// Builds a `GlyphBrush` in the given `glow::Context`.
    pub fn build<C>(self, context: &mut C) -> GlyphBrush<C::Backend, F, H>
    where
        C: GraphicsContext,
        C::Backend: backend::texture::Texture<Dim2, NormR8UI>
            + backend::shader::Shader
            + backend::tess::Tess<(), u32, Instance, Interleaved>
            + backend::pipeline::PipelineBase
            + backend::pipeline::PipelineTexture<Dim2, NormR8UI>
            + backend::render_gate::RenderGate
            + backend::tess_gate::TessGate<(), u32, Instance, Interleaved>,
        [[f32; 4]; 4]: backend::shader::Uniformable<C::Backend>,
        TextureBinding<Dim2, NormUnsigned>: backend::shader::Uniformable<C::Backend>,
    {
        GlyphBrush::new(context, self.inner)
    }
}
