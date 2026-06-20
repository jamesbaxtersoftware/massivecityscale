//! Pixel-art post-process (decision D1): render the 3D scene into a small image,
//! then upscale it to the window with nearest-neighbour sampling so everything
//! reads as chunky pixels — keeping the full 3D engine underneath.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

/// Internal render resolution (16:9). Lower = chunkier pixels.
pub const LORES_W: u32 = 480;
pub const LORES_H: u32 = 270;

pub struct PixelatePlugin;

impl Plugin for PixelatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_pixelate);
    }
}

/// One-shot: retarget the 3D camera to a low-res image and blit it full-screen,
/// nearest-sampled, via a 2D camera. Runs once the 3D camera exists.
fn setup_pixelate(
    mut done: Local<bool>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cam3d: Query<&mut Camera, With<Camera3d>>,
) {
    if *done {
        return;
    }
    let Ok(mut camera) = cam3d.get_single_mut() else {
        return;
    };

    let size = Extent3d {
        width: LORES_W,
        height: LORES_H,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);

    // 3D scene renders into the low-res image instead of the window.
    camera.target = RenderTarget::Image(handle.clone());
    camera.order = 0;

    // 2D camera draws the upscaled result to the window (over the 3D, order 1).
    commands.spawn((Camera2d, Camera { order: 1, ..default() }));

    // Full-screen nearest-sampled blit of the low-res scene; behind any HUD/menu.
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode::new(handle),
        ZIndex(-100),
    ));

    *done = true;
}
