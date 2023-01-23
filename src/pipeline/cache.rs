use luminance::{
    backend,
    context::GraphicsContext,
    pixel::NormR8UI,
    texture::{Dim2, MagFilter, MinFilter, Sampler, TexelUpload, Texture, Wrap},
};

pub struct Cache<B>
where
    B: ?Sized + backend::texture::Texture<Dim2, NormR8UI>,
{
    pub(crate) texture: Texture<B, Dim2, NormR8UI>,
}

impl<B> Cache<B>
where
    B: ?Sized + backend::texture::Texture<Dim2, NormR8UI>,
{
    pub fn new<C>(context: &mut C, width: u32, height: u32) -> Self
    where
        C: GraphicsContext<Backend = B>,
    {
        let texels = &vec![0; (width * height) as usize][..];
        let texture = context
            .new_texture(
                [width, height],
                Sampler {
                    wrap_r: Wrap::ClampToEdge,
                    wrap_s: Wrap::ClampToEdge,
                    wrap_t: Wrap::ClampToEdge,
                    min_filter: MinFilter::Linear,
                    mag_filter: MagFilter::Linear,
                    depth_comparison: None,
                },
                TexelUpload::BaseLevel { texels, mipmaps: 0 },
            )
            .expect("failed to create texture");

        Cache { texture }

        // gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        // let texture = {
        //     let handle = gl.create_texture().expect("Create glyph cache texture");

        //     gl.bind_texture(glow::TEXTURE_2D, Some(handle));

        //     gl.tex_parameter_i32(
        //         glow::TEXTURE_2D,
        //         glow::TEXTURE_WRAP_S,
        //         glow::CLAMP_TO_EDGE as i32,
        //     );
        //     gl.tex_parameter_i32(
        //         glow::TEXTURE_2D,
        //         glow::TEXTURE_WRAP_T,
        //         glow::CLAMP_TO_EDGE as i32,
        //     );
        //     gl.tex_parameter_i32(
        //         glow::TEXTURE_2D,
        //         glow::TEXTURE_MIN_FILTER,
        //         glow::LINEAR as i32,
        //     );
        //     gl.tex_parameter_i32(
        //         glow::TEXTURE_2D,
        //         glow::TEXTURE_MAG_FILTER,
        //         glow::LINEAR as i32,
        //     );

        //     gl.tex_image_2d(
        //         glow::TEXTURE_2D,
        //         0,
        //         glow::R8 as i32,
        //         width as i32,
        //         height as i32,
        //         0,
        //         glow::RED,
        //         glow::UNSIGNED_BYTE,
        //         None,
        //     );
        //     gl.bind_texture(glow::TEXTURE_2D, None);

        //     handle
        // };

        // Cache { texture }
    }

    pub fn update(&mut self, offset: [u16; 2], size: [u16; 2], data: &[u8]) {
        let offset = [offset[0] as u32, offset[1] as u32];
        let size = [size[0] as u32, size[1] as u32];

        self.texture
            .upload_part_raw(
                offset,
                size,
                TexelUpload::BaseLevel {
                    texels: data,
                    mipmaps: 0,
                },
            )
            .expect("failed to upload to texture region");

        // let [offset_x, offset_y] = offset;
        // let [width, height] = size;

        // gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));

        // gl.tex_sub_image_2d_u8_slice(
        //     glow::TEXTURE_2D,
        //     0,
        //     i32::from(offset_x),
        //     i32::from(offset_y),
        //     i32::from(width),
        //     i32::from(height),
        //     glow::RED,
        //     glow::UNSIGNED_BYTE,
        //     Some(data),
        // );

        // gl.bind_texture(glow::TEXTURE_2D, None);
    }
}
