use std::{f32::consts::PI, io::Cursor};

use fontdue::Font;
use image::{Rgb, Rgba, RgbaImage, imageops};
use imageproc::geometric_transformations::Interpolation;
use rand::{Rng, rng, seq::IndexedRandom};
use raqote::{Color, DrawOptions, DrawTarget, PathBuilder, SolidSource, Source, StrokeStyle};

pub struct Config {
    pub length: u32,
    pub width: u32,
    pub height: u32,
    pub color: [u8; 3],
    pub background_color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            length: 4,
            width: 240,
            height: 80,
            color: [0, 0, 0],
            background_color: [255, 255, 255],
        }
    }
}

impl Config {
    pub fn generate(&self) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
        let charset: Vec<char> = "23456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz"
            .chars()
            .collect();
        let mut rng = rng();
        let captcha_text: String = (0..self.length)
            .map(|_| *charset.choose(&mut rng).unwrap())
            .collect();

        let font_data = std::fs::read("Arial.ttf")?;
        let font = Font::from_bytes(font_data, fontdue::FontSettings::default())?;

        let font_size = (self.width / self.length).min(self.height);

        let width = self.width;
        let height = self.height;

        let background_color = Rgba([
            self.background_color[0],
            self.background_color[1],
            self.background_color[2],
            255,
        ]);

        let mut img = RgbaImage::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = background_color
        }

        let rasterized_fonts = captcha_text
            .chars()
            .map(|c| font.rasterize(c, font_size as f32))
            .collect::<Vec<_>>();

        let fonts_width: f32 = rasterized_fonts.iter().map(|x| x.0.advance_width).sum();
        let spacing = (self.width as f32 - fonts_width) / (self.length as f32 + 1.0);

        let mut x_offset = spacing; // 起始 X 位置

        for (metrics, bitmap) in rasterized_fonts {
            let mut rgba_data = Vec::with_capacity((metrics.width * metrics.height * 4) as usize);
            for alpha in bitmap {
                rgba_data.push(self.color[0]);
                rgba_data.push(self.color[1]);
                rgba_data.push(self.color[2]);
                rgba_data.push(alpha);
            }

            let font_img =
                RgbaImage::from_raw(metrics.width as u32, metrics.height as u32, rgba_data)
                    .unwrap();

            let rotate_angle = (PI / 8.0) * rng.random_range(-1.0..1.0);

            let (rotated_width, rotated_height) =
                rotated_rect_size(metrics.width as f32, metrics.height as f32, rotate_angle);

            let mut expanded = RgbaImage::new(rotated_width as u32, rotated_height as u32);
            imageops::overlay(
                &mut expanded,
                &font_img,
                ((rotated_width as u32 - font_img.width()) / 2) as i64,
                ((rotated_height as u32 - font_img.height()) / 2) as i64,
            );

            let rotated = imageproc::geometric_transformations::rotate_about_center(
                &mut expanded,
                rotate_angle,
                Interpolation::Bilinear,
                Rgba([255, 255, 255, 255]),
            );

            let px = (x_offset as i64) - (rotated_width as i64 - font_img.width() as i64) / 2;
            let py = ((self.height as f32 - rotated.height() as f32) / 2.0) as i64;
            imageops::overlay(&mut img, &rotated, px, py);

            x_offset += (metrics.advance_width as f32) as f32 + spacing as f32;
        }

        // imageproc::noise::gaussian_noise_mut(&mut img, 0.0, 50.0, 50);

        for _ in 0..5 {
            draw_line(&mut img);
        }

        for _ in 0..2 {
            draw_cubic_line(&mut img);
        }

        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, image::ImageFormat::Png).unwrap();

        Ok((captcha_text, buffer.into_inner()))
    }

    #[cfg(feature = "base64")]
    pub fn generate_base64(&self) -> Result<(String, String), Box<dyn std::error::Error>> {
        use base64::{Engine, engine::general_purpose};

        let (text, buffer) = self.generate()?;

        let base64_string = general_purpose::STANDARD.encode(buffer);

        Ok((text, base64_string))
    }
}

fn rotated_rect_size(width: f32, height: f32, angle: f32) -> (f32, f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // 原图的四个角相对于中心的偏移
    let hw = width / 2.0;
    let hh = height / 2.0;

    // 四个角点
    let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for &(x, y) in &corners {
        // 旋转每个角点
        let rx = x * cos_a - y * sin_a;
        let ry = x * sin_a + y * cos_a;
        min_x = min_x.min(rx);
        max_x = max_x.max(rx);
        min_y = min_y.min(ry);
        max_y = max_y.max(ry);
    }

    let rotated_width = max_x - min_x;
    let rotated_height = max_y - min_y;

    (rotated_width, rotated_height)
}

fn merge(img: &mut RgbaImage, dt: DrawTarget) {
    let width = dt.width();
    let height = dt.height();

    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);

    for color in dt.into_vec() {
        let a = (color >> 24 & 0xFF) as u8;
        let r = (color >> 16 & 0xFF) as u8;
        let g = (color >> 8 & 0xFF) as u8;
        let b = (color >> 0 & 0xFF) as u8;
        rgba_data.push(r);
        rgba_data.push(g);
        rgba_data.push(b);
        rgba_data.push(a);
    }

    let font_img = RgbaImage::from_raw(width as u32, height as u32, rgba_data).unwrap();

    imageops::overlay(img, &font_img, 0, 0);
}

fn random_color() -> Rgb<u8> {
    let mut rng = rand::rng();

    let r = rng.random_range(0..=255);
    let g = rng.random_range(0..=255);
    let b = rng.random_range(0..=255);

    Rgb([r, g, b])
}

fn draw_line(img: &mut RgbaImage) {
    let mut rng = rand::rng();
    let width = img.width();
    let height = img.height();

    let x1 = rng.random_range(0..width);
    let y1 = rng.random_range(0..height);
    let x2 = rng.random_range(0..width);
    let y2 = rng.random_range(0..height);

    let mut dt = DrawTarget::new(width.try_into().unwrap(), height.try_into().unwrap());
    let mut pb = PathBuilder::new();

    pb.move_to(x1 as f32, y1 as f32);
    pb.line_to(x2 as f32, y2 as f32);
    let path = pb.finish();

    let color = random_color();

    dt.stroke(
        &path,
        &Source::Solid(SolidSource::from(Color::new(
            255, color.0[0], color.0[1], color.0[2],
        ))),
        &StrokeStyle::default(),
        &DrawOptions::new(),
    );

    merge(img, dt);
}

fn draw_cubic_line(img: &mut RgbaImage) {
    let mut rng = rand::rng();
    let width = img.width();
    let height = img.height();

    let x1 = 0;
    let y1 = rng.random_range(0..height);
    let x2 = width;
    let y2 = rng.random_range(0..height);

    let cx = rng.random_range((width / 4)..(width / 4 * 3));
    let cy = rng.random_range(0..height);

    let mut dt = DrawTarget::new(width.try_into().unwrap(), height.try_into().unwrap());
    let mut pb = PathBuilder::new();

    pb.move_to(x1 as f32, y1 as f32);
    pb.cubic_to(
        cx as f32, cy as f32, cx as f32, cy as f32, x2 as f32, y2 as f32,
    );
    let path = pb.finish();

    let color = random_color();

    dt.stroke(
        &path,
        &Source::Solid(SolidSource::from(Color::new(
            128, color.0[0], color.0[1], color.0[2],
        ))),
        &StrokeStyle::default(),
        &DrawOptions::new(),
    );

    merge(img, dt);
}
