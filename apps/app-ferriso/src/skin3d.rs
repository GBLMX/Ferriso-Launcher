//! A small CPU renderer for the Minecraft player model.
//!
//! The original web preview depends on WebGL. This module keeps the same
//! interaction model native: textured boxes, a perspective-free orbit camera,
//! a depth buffer, and nearest-neighbour sampling for the pixel-art skin.

use image::{Rgba, RgbaImage};

const TEX: f32 = 64.0;
pub const WIDTH: u32 = 340;
pub const HEIGHT: u32 = 420;
const OUT_W: u32 = WIDTH;
const OUT_H: u32 = HEIGHT;

type Rect = (u32, u32, u32, u32);

#[derive(Clone, Copy)]
struct FaceUv {
    rect: Rect,
    shade: f32,
}

#[derive(Clone, Copy)]
struct ModelBox {
    center: [f32; 3],
    size: [f32; 3],
    faces: [FaceUv; 6],
}

fn face(rect: Rect, shade: f32) -> FaceUv {
    FaceUv { rect, shade }
}

fn standard_faces(
    right: Rect,
    left: Rect,
    top: Rect,
    bottom: Rect,
    front: Rect,
    back: Rect,
) -> [FaceUv; 6] {
    [
        face(right, 0.78),
        face(left, 0.78),
        face(top, 1.0),
        face(bottom, 0.52),
        face(front, 0.94),
        face(back, 0.64),
    ]
}

fn build_model(slim: bool) -> Vec<ModelBox> {
    let mut boxes = Vec::with_capacity(18);

    // Feet at y=-16, head top at y=16. The skin's front is +Z.
    let head = standard_faces(
        (0, 8, 8, 8),
        (16, 8, 8, 8),
        (8, 0, 8, 8),
        (16, 0, 8, 8),
        (8, 8, 8, 8),
        (24, 8, 8, 8),
    );
    let body = standard_faces(
        (16, 20, 4, 12),
        (28, 20, 4, 12),
        (20, 16, 8, 4),
        (28, 16, 8, 4),
        (20, 20, 8, 12),
        (32, 20, 8, 12),
    );
    let right_leg = standard_faces(
        (0, 20, 4, 12),
        (8, 20, 4, 12),
        (4, 16, 4, 4),
        (8, 16, 4, 4),
        (4, 20, 4, 12),
        (12, 20, 4, 12),
    );
    let left_leg = standard_faces(
        (16, 52, 4, 12),
        (24, 52, 4, 12),
        (20, 48, 4, 4),
        (24, 48, 4, 4),
        (20, 52, 4, 12),
        (28, 52, 4, 12),
    );

    let arm_width = if slim { 3.0 } else { 4.0 };
    let arm_x = 44;
    let right_arm_uv = standard_faces(
        (40, 20, 4, 12),
        (arm_x + 3, 20, 4, 12),
        (arm_x, 16, arm_width as u32, 4),
        (arm_x + arm_width as u32, 16, arm_width as u32, 4),
        (arm_x, 20, arm_width as u32, 12),
        (arm_x + arm_width as u32, 20, arm_width as u32, 12),
    );
    let left_arm_uv = standard_faces(
        (32, 52, 4, 12),
        (40, 52, 4, 12),
        (36, 48, arm_width as u32, 4),
        (36 + arm_width as u32, 48, arm_width as u32, 4),
        (36, 52, arm_width as u32, 12),
        (40, 52, arm_width as u32, 12),
    );

    boxes.push(ModelBox {
        center: [-6.0, 2.0, 0.0],
        size: [arm_width, 12.0, 4.0],
        faces: right_arm_uv,
    });
    boxes.push(ModelBox {
        center: [6.0, 2.0, 0.0],
        size: [arm_width, 12.0, 4.0],
        faces: left_arm_uv,
    });
    boxes.push(ModelBox {
        center: [-2.0, -10.0, 0.0],
        size: [4.0, 12.0, 4.0],
        faces: right_leg,
    });
    boxes.push(ModelBox {
        center: [2.0, -10.0, 0.0],
        size: [4.0, 12.0, 4.0],
        faces: left_leg,
    });
    boxes.push(ModelBox {
        center: [0.0, 2.0, 0.0],
        size: [8.0, 12.0, 4.0],
        faces: body,
    });
    boxes.push(ModelBox {
        center: [0.0, 12.0, 0.0],
        size: [8.0, 8.0, 8.0],
        faces: head,
    });

    // Second skin layer. The overlay boxes are inflated so they never fight
    // with the base layer in the depth buffer.
    let hat = standard_faces(
        (32, 8, 8, 8),
        (48, 8, 8, 8),
        (40, 0, 8, 8),
        (48, 0, 8, 8),
        (40, 8, 8, 8),
        (56, 8, 8, 8),
    );
    let jacket = standard_faces(
        (16, 36, 4, 12),
        (28, 36, 4, 12),
        (20, 32, 8, 4),
        (28, 32, 8, 4),
        (20, 36, 8, 12),
        (32, 36, 8, 12),
    );
    let right_sleeve = standard_faces(
        (40, 36, 4, 12),
        (arm_x + 3, 36, 4, 12),
        (arm_x, 32, arm_width as u32, 4),
        (arm_x + arm_width as u32, 32, arm_width as u32, 4),
        (arm_x, 36, arm_width as u32, 12),
        (arm_x + arm_width as u32, 36, arm_width as u32, 12),
    );
    let left_sleeve = standard_faces(
        (48, 52, 4, 12),
        (56, 52, 4, 12),
        (52, 48, arm_width as u32, 4),
        (52 + arm_width as u32, 48, arm_width as u32, 4),
        (52, 52, arm_width as u32, 12),
        (56, 52, arm_width as u32, 12),
    );
    let right_pants = standard_faces(
        (0, 52, 4, 12),
        (8, 52, 4, 12),
        (4, 48, 4, 4),
        (8, 48, 4, 4),
        (4, 52, 4, 12),
        (12, 52, 4, 12),
    );
    let left_pants = right_pants;

    boxes.push(ModelBox {
        center: [-6.0, 2.0, 0.0],
        size: [arm_width + 0.6, 12.6, 4.6],
        faces: right_sleeve,
    });
    boxes.push(ModelBox {
        center: [6.0, 2.0, 0.0],
        size: [arm_width + 0.6, 12.6, 4.6],
        faces: left_sleeve,
    });
    boxes.push(ModelBox {
        center: [-2.0, -10.0, 0.0],
        size: [4.6, 12.6, 4.6],
        faces: right_pants,
    });
    boxes.push(ModelBox {
        center: [2.0, -10.0, 0.0],
        size: [4.6, 12.6, 4.6],
        faces: left_pants,
    });
    boxes.push(ModelBox {
        center: [0.0, 2.0, 0.0],
        size: [8.6, 12.6, 4.6],
        faces: jacket,
    });
    boxes.push(ModelBox {
        center: [0.0, 12.0, 0.0],
        size: [8.6, 8.6, 8.6],
        faces: hat,
    });

    boxes
}

#[derive(Clone, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32,
}

fn project(
    point: [f32; 3],
    yaw: f32,
    pitch: f32,
    scale: f32,
    cx: f32,
    cy: f32,
) -> Vertex {
    let (sy, cyaw) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (x, y, z) = (point[0], point[1], point[2]);
    let x1 = x * cyaw + z * sy;
    let z1 = -x * sy + z * cyaw;
    let y1 = y * cp - z1 * sp;
    let z2 = y * sp + z1 * cp;
    Vertex {
        x: cx + x1 * scale,
        y: cy - y1 * scale,
        z: z2,
        u: 0.0,
        v: 0.0,
    }
}

fn corners(center: [f32; 3], size: [f32; 3], face: usize) -> [[f32; 3]; 4] {
    let (x, y, z) = (center[0], center[1], center[2]);
    let (hx, hy, hz) = (size[0] / 2.0, size[1] / 2.0, size[2] / 2.0);
    match face {
        // +X
        0 => [
            [x + hx, y + hy, z + hz],
            [x + hx, y - hy, z + hz],
            [x + hx, y - hy, z - hz],
            [x + hx, y + hy, z - hz],
        ],
        // -X
        1 => [
            [x - hx, y + hy, z - hz],
            [x - hx, y - hy, z - hz],
            [x - hx, y - hy, z + hz],
            [x - hx, y + hy, z + hz],
        ],
        // +Y
        2 => [
            [x - hx, y + hy, z - hz],
            [x - hx, y + hy, z + hz],
            [x + hx, y + hy, z + hz],
            [x + hx, y + hy, z - hz],
        ],
        // -Y
        3 => [
            [x - hx, y - hy, z + hz],
            [x - hx, y - hy, z - hz],
            [x + hx, y - hy, z - hz],
            [x + hx, y - hy, z + hz],
        ],
        // +Z
        4 => [
            [x - hx, y + hy, z + hz],
            [x - hx, y - hy, z + hz],
            [x + hx, y - hy, z + hz],
            [x + hx, y + hy, z + hz],
        ],
        // -Z
        _ => [
            [x + hx, y + hy, z - hz],
            [x + hx, y - hy, z - hz],
            [x - hx, y - hy, z - hz],
            [x - hx, y + hy, z - hz],
        ],
    }
}

fn sample(texture: &RgbaImage, rect: Rect, u: f32, v: f32) -> Rgba<u8> {
    let x = (rect.0 as f32 + u * rect.2 as f32)
        .clamp(0.0, texture.width() as f32 - 1.0);
    let y = (rect.1 as f32 + v * rect.3 as f32)
        .clamp(0.0, texture.height() as f32 - 1.0);
    let sx = ((x / TEX) * texture.width() as f32)
        .clamp(0.0, texture.width() as f32 - 1.0) as u32;
    let sy = ((y / TEX) * texture.height() as f32)
        .clamp(0.0, texture.height() as f32 - 1.0) as u32;
    *texture.get_pixel(sx, sy)
}

fn edge(a: Vertex, b: Vertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn raster_triangle(
    vertices: &[Vertex; 4],
    tri: [usize; 3],
    texture: &RgbaImage,
    rect: Rect,
    shade: f32,
    pixels: &mut [u8],
    depth: &mut [f32],
) {
    let a = vertices[tri[0]];
    let mut b = vertices[tri[1]];
    let mut c = vertices[tri[2]];
    let mut area = edge(a, b, c.x, c.y);
    if area.abs() < 0.01 {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut b, &mut c);
        area = -area;
    }

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(OUT_W as f32 - 1.0) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(OUT_H as f32 - 1.0) as u32;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let x = px as f32 + 0.5;
            let y = py as f32 + 0.5;
            let w0 = edge(b, c, x, y) / area;
            let w1 = edge(c, a, x, y) / area;
            let w2 = edge(a, b, x, y) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = a.z * w0 + b.z * w1 + c.z * w2;
            let index = (py * OUT_W + px) as usize;
            if z <= depth[index] {
                continue;
            }
            let u = a.u * w0 + b.u * w1 + c.u * w2;
            let v = a.v * w0 + b.v * w1 + c.v * w2;
            let color = sample(texture, rect, u, v);
            if color[3] == 0 {
                continue;
            }
            let alpha = color[3] as f32 / 255.0;
            let target = index * 4;
            let blend = |dst: u8, src: u8| {
                (src as f32 * shade * alpha + dst as f32 * (1.0 - alpha))
                    .clamp(0.0, 255.0) as u8
            };
            if alpha >= 0.999 {
                pixels[target] = blend(0, color[0]);
                pixels[target + 1] = blend(0, color[1]);
                pixels[target + 2] = blend(0, color[2]);
                pixels[target + 3] = 255;
            } else {
                pixels[target] = blend(pixels[target], color[0]);
                pixels[target + 1] = blend(pixels[target + 1], color[1]);
                pixels[target + 2] = blend(pixels[target + 2], color[2]);
                pixels[target + 3] = (pixels[target + 3] as f32
                    + (255.0 - pixels[target + 3] as f32) * alpha)
                    as u8;
            }
            depth[index] = z;
        }
    }
}

fn draw_box(
    model: &ModelBox,
    yaw: f32,
    pitch: f32,
    scale: f32,
    texture: &RgbaImage,
    pixels: &mut [u8],
    depth: &mut [f32],
) {
    let cx = OUT_W as f32 / 2.0;
    let cy = OUT_H as f32 / 2.0 + 18.0;
    for (index, face_uv) in model.faces.iter().enumerate() {
        let quad = corners(model.center, model.size, index);
        let mut vertices = [Vertex {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            u: 0.0,
            v: 0.0,
        }; 4];
        for (corner, vertex) in quad.iter().zip(vertices.iter_mut()) {
            *vertex = project(*corner, yaw, pitch, scale, cx, cy);
        }
        let uvs = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)];
        for (vertex, (u, v)) in vertices.iter_mut().zip(uvs) {
            vertex.u = u;
            vertex.v = v;
        }
        raster_triangle(
            &vertices,
            [0, 1, 2],
            texture,
            face_uv.rect,
            face_uv.shade,
            pixels,
            depth,
        );
        raster_triangle(
            &vertices,
            [0, 2, 3],
            texture,
            face_uv.rect,
            face_uv.shade,
            pixels,
            depth,
        );
    }
}

/// Render the selected skin into an RGBA buffer. `yaw` and `pitch` are radians;
/// callers drive them from pointer drags.
pub fn render_skin(
    skin_bytes: &[u8],
    cape_bytes: Option<&[u8]>,
    slim: bool,
    yaw: f32,
    pitch: f32,
) -> Option<RgbaImage> {
    let skin = decode_texture(skin_bytes)?;
    let cape = cape_bytes.and_then(decode_texture);
    let mut pixels = vec![0u8; (OUT_W * OUT_H * 4) as usize];
    let mut depth = vec![f32::NEG_INFINITY; (OUT_W * OUT_H) as usize];

    if let Some(cape) = cape.as_ref() {
        let cape_box = ModelBox {
            center: [0.0, 2.0, -2.8],
            size: [10.0, 16.0, 1.0],
            faces: standard_faces(
                (1, 1, 10, 16),
                (1, 1, 10, 16),
                (1, 1, 10, 16),
                (1, 1, 10, 16),
                (1, 1, 10, 16),
                (1, 1, 10, 16),
            ),
        };
        draw_box(&cape_box, yaw, pitch, 9.0, cape, &mut pixels, &mut depth);
    }

    for model in build_model(slim) {
        draw_box(&model, yaw, pitch, 9.0, &skin, &mut pixels, &mut depth);
    }

    RgbaImage::from_raw(OUT_W, OUT_H, pixels)
}

/// Decoding a 64x64 skin is cheap in absolute terms but happens on every
/// pointer move while dragging. Keep the last few decoded textures so the
/// rasterizer only does pixel work per frame.
fn decode_texture(bytes: &[u8]) -> Option<RgbaImage> {
    use std::{
        collections::{HashMap, VecDeque},
        hash::{Hash, Hasher},
        sync::{Mutex, OnceLock},
    };

    struct Cache {
        entries: HashMap<u64, RgbaImage>,
        order: VecDeque<u64>,
    }

    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let key = hasher.finish();
    let cache = CACHE.get_or_init(|| {
        Mutex::new(Cache {
            entries: HashMap::new(),
            order: VecDeque::new(),
        })
    });
    let mut cache = cache.lock().ok()?;
    if let Some(image) = cache.entries.get(&key) {
        return Some(image.clone());
    }
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    if cache.entries.len() >= 8
        && let Some(oldest) = cache.order.pop_front()
    {
        cache.entries.remove(&oldest);
    }
    cache.entries.insert(key, image.clone());
    cache.order.push_back(key);
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::render_skin;
    use image::{
        ExtendedColorType, ImageEncoder, Rgba, RgbaImage,
        codecs::png::PngEncoder,
    };

    fn png() -> Vec<u8> {
        let mut image = RgbaImage::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                let color = if y < 32 {
                    if x < 32 {
                        [220, 60, 60, 255]
                    } else {
                        [60, 160, 220, 255]
                    }
                } else if x < 32 {
                    [60, 200, 120, 255]
                } else {
                    [230, 190, 60, 255]
                };
                image.put_pixel(x, y, Rgba(color));
            }
        }
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(image.as_raw(), 64, 64, ExtendedColorType::Rgba8)
            .expect("encode test skin");
        bytes
    }

    fn visible(image: &RgbaImage) -> usize {
        image.pixels().filter(|pixel| pixel[3] > 0).count()
    }

    #[test]
    fn renders_non_empty_frames_while_orbiting() {
        let skin = png();
        let mut previous = None;
        for yaw in [0.0, 0.5, 1.5, 3.0] {
            let image = render_skin(&skin, None, false, yaw, 0.12)
                .expect("skin decodes");
            assert!(visible(&image) > 5_000, "empty frame at yaw {yaw}");
            if let Some(previous) = previous {
                assert_ne!(
                    previous, image,
                    "rotation did not change the frame"
                );
            }
            previous = Some(image);
        }
    }
}
