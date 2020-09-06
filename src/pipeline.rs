mod cache;

use crate::ab_glyph::{point, Rect};
use crate::Region;
use cache::Cache;

use luminance::pipeline::PipelineError;
use luminance_derive::UniformInterface;
use luminance_derive::{Semantics, Vertex};
use luminance_front::blending::{Blending, Equation, Factor};
use luminance_front::context::GraphicsContext;
use luminance_front::pipeline::{Pipeline as LuminancePipeline, TextureBinding};
use luminance_front::pixel::NormUnsigned;
use luminance_front::render_state::RenderState;
use luminance_front::shader::{Program, Uniform};
use luminance_front::shading_gate::ShadingGate;
use luminance_front::tess::{Interleaved, Mode, Tess, TessBuilder};
use luminance_front::texture::Dim2;

type VertexIndex = u32;

pub struct Pipeline {
    program: Program<Semantics, (), ShaderInterface>,
    vertex_array: Option<Tess<(), VertexIndex, Instance, Interleaved>>,
    cache: Cache,
}

const VS: &str = include_str!("./shaders/vertex.glsl");
const FS: &str = include_str!("./shaders/fragment.glsl");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum Semantics {
    #[sem(name = "left_top", repr = "[f32; 3]", wrapper = "LeftTop")]
    LeftTop,
    #[sem(name = "right_bottom", repr = "[f32; 2]", wrapper = "RightBottom")]
    RightBottom,
    #[sem(name = "tex_left_top", repr = "[f32; 2]", wrapper = "TexLeftTop")]
    TexLeftTop,
    #[sem(
        name = "tex_right_bottom",
        repr = "[f32; 2]",
        wrapper = "TexRightBottom"
    )]
    TexRightBottom,
    #[sem(name = "color", repr = "[f32; 4]", wrapper = "VertexColor")]
    Color,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "Semantics", instanced = "true")]
pub struct Instance {
    left_top: LeftTop,
    right_bottom: RightBottom,
    tex_left_top: TexLeftTop,
    tex_right_bottom: TexRightBottom,
    color: VertexColor,
}

impl Instance {
    pub fn from_vertex(
        glyph_brush::GlyphVertex {
            mut tex_coords,
            pixel_coords,
            bounds,
            extra,
        }: glyph_brush::GlyphVertex,
    ) -> Instance {
        let gl_bounds = bounds;

        let mut gl_rect = Rect {
            min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
            max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
        };

        // handle overlapping bounds, modify uv_rect to preserve texture aspect
        if gl_rect.max.x > gl_bounds.max.x {
            let old_width = gl_rect.width();
            gl_rect.max.x = gl_bounds.max.x;
            tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.min.x < gl_bounds.min.x {
            let old_width = gl_rect.width();
            gl_rect.min.x = gl_bounds.min.x;
            tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.max.y > gl_bounds.max.y {
            let old_height = gl_rect.height();
            gl_rect.max.y = gl_bounds.max.y;
            tex_coords.max.y =
                tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
        }

        if gl_rect.min.y < gl_bounds.min.y {
            let old_height = gl_rect.height();
            gl_rect.min.y = gl_bounds.min.y;
            tex_coords.min.y =
                tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
        }

        Instance {
            left_top: LeftTop::from([gl_rect.min.x, gl_rect.max.y, extra.z]),
            right_bottom: RightBottom::from([gl_rect.max.x, gl_rect.min.y]),
            tex_left_top: TexLeftTop::from([tex_coords.min.x, tex_coords.max.y]),
            tex_right_bottom: TexRightBottom::from([tex_coords.max.x, tex_coords.min.y]),
            color: VertexColor::from(extra.color),
        }
    }
}

#[derive(UniformInterface)]
struct ShaderInterface {
    transform: Uniform<[[f32; 4]; 4]>,
    font_sampler: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

impl Pipeline {
    pub fn new<C>(ctx: &mut C, cache_width: u32, cache_height: u32) -> Self
    where
        C: GraphicsContext<Backend = luminance_front::Backend>,
    {
        let cache = Cache::new(ctx, cache_width, cache_height);

        let program = ctx
            .new_shader_program::<Semantics, (), ShaderInterface>()
            .from_strings(VS, None, None, FS)
            .expect("shader failed to compile")
            .program;

        Pipeline {
            program,
            cache,
            vertex_array: None,
        }
    }

    pub fn draw<'a>(
        &mut self,
        pipeline: &mut LuminancePipeline<'a>,
        shading_gate: &mut ShadingGate<'a>,
        transform: [f32; 16],
        _region: Option<Region>,
    ) -> Result<(), PipelineError> {
        if let Some(vao) = &self.vertex_array {
            let bound_texture = pipeline.bind_texture(&mut self.cache.texture)?;

            // Start shading with our program.
            shading_gate.shade(&mut self.program, |mut iface, uni, mut rdr_gate| {
                iface.set(&uni.transform, to_4x4(&transform));
                iface.set(&uni.font_sampler, bound_texture.binding());

                // Start rendering things with the default render state provided by luminance.
                rdr_gate.render(
                    &RenderState::default().set_blending(Blending {
                        equation: Equation::Additive,
                        src: Factor::SrcAlpha,
                        dst: Factor::SrcAlphaComplement,
                    }),
                    |mut tess_gate| tess_gate.render(vao),
                )
            })
        } else {
            Ok(())
        }
    }

    pub fn update_cache(&mut self, offset: [u16; 2], size: [u16; 2], data: &[u8]) {
        self.cache.update(offset, size, data);
    }

    pub fn increase_cache_size<C>(&mut self, ctx: &mut C, width: u32, height: u32)
    where
        C: GraphicsContext<Backend = luminance_front::Backend>,
    {
        self.cache = Cache::new(ctx, width, height);
    }

    pub fn upload<C>(&mut self, ctx: &mut C, instances: &[Instance])
    where
        C: GraphicsContext<Backend = luminance_front::Backend>,
    {
        self.vertex_array = Some(
            TessBuilder::new(ctx)
                .set_instances(instances)
                .set_vertex_nb(4)
                .set_mode(Mode::TriangleStrip)
                .build()
                .unwrap(),
        );
    }
}

// From: https://github.com/rust-lang/rfcs/issues/1833
fn to_4x4(array: &[f32; 16]) -> [[f32; 4]; 4] {
    unsafe { *(array as *const _ as *const _) }
}
