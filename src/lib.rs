#![warn(clippy::pedantic, clippy::nursery)]
#![cfg_attr(not(test), no_std)]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rawwood_12() {
        for i in 0..5 {
            let raw_wood = wood(584, 668, 40., 12., &WOOD_1);
            raw_wood
                .unwrap()
                .save(format!("rawwood_12_{}.png", i))
                .unwrap();
        }
    }
    #[test]
    fn brightwood12() {
        for i in 0..5 {
            let bright = wood(584, 668, 40., 12., &BRIGHT_WOOD);
            bright
                .unwrap()
                .save(format!("brightwood_12_{}.png", i))
                .unwrap();
        }
    }

    #[test]
    fn rawwood_24() {
        for i in 0..5 {
            let raw_wood = wood(584, 668, 40., 24., &WOOD_1);
            raw_wood
                .unwrap()
                .save(format!("rawwood_24_{}.png", i))
                .unwrap();
        }
    }
}

extern crate alloc;
use alloc::vec::Vec;

struct Noise {
    width: usize,
    height: usize,
    data: Vec<Vec<f64>>,
}

use rand::distributions::{Distribution, Uniform};

impl Noise {
    fn gen_noise(width: usize, height: usize) -> Self {
        /* algorithm taken from https://lodev.org/cgtutor/randomnoise.html#Wood */
        let between = Uniform::from(0.0..1.0);
        let mut rng = rand::thread_rng();
        let mut noise: Vec<Vec<f64>> = Vec::new();
        for _ in 0..height {
            let mut vec = Vec::new();
            for _ in 0..width {
                vec.push(between.sample(&mut rng));
            }
            noise.push(vec);
        }

        Self {
            width,
            height,
            data: noise,
        }
    }

    fn sample_smooth_noise(&self, x: f64, y: f64) -> f64 {
        /* algorithm taken from https://lodev.org/cgtutor/randomnoise.html#Wood */
        let fract_x = x.fract();
        let fract_y = y.fract();
        let width = self.width;
        let height = self.height;

        //wrap around
        let x1: usize = ((x as usize) + width) % width;
        let y1: usize = ((y as usize) + height) % height;

        //neighbor values
        let x2: usize = (x1 + width - 1) % width;
        let y2: usize = (y1 + height - 1) % height;

        //smooth the noise with bilinear interpolation
        let mut value = 0.0;
        value += fract_x * fract_y * self.data[y1][x1];
        value += (1. - fract_x) * fract_y * self.data[y1][x2];
        value += fract_x * (1. - fract_y) * self.data[y2][x1];
        value += (1. - fract_x) * (1. - fract_y) * self.data[y2][x2];

        value
    }

    fn turbulence(&self, x: f64, y: f64, initial_size: f64) -> f64 {
        /* algorithm taken from https://lodev.org/cgtutor/randomnoise.html#Wood */
        let mut value = 0.0_f64;
        let mut size = initial_size;

        while size >= 1. {
            value += self.sample_smooth_noise(x / size, y / size) * size;
            size /= 2.0;
        }

        128.0 * value / initial_size
    }
}

pub struct WoodProfile {
    brightness_adjustment: i32,
    dark_color: [u8; 3],
    light_color: [u8; 3],
}

pub const BRIGHT_WOOD: WoodProfile = WoodProfile {
    brightness_adjustment: 20,
    dark_color: [120, 70, 70],
    light_color: [208, 158, 70],
};

pub const WOOD_1: WoodProfile = WoodProfile {
    brightness_adjustment: 0,
    dark_color: [120, 70, 70],
    light_color: [208, 158, 70],
};

/// * `width`: width of the image to be generated
/// * `height`: height of the image to be generated
/// * `offsetstdev`: signifies how large the offset should be (the center of the wood grain is randomly shifted in the x and y directions).
/// * `length_scale`: denotes the average length of spacing between grains in pixels.
///
/// # Errors
/// Returns `BadVariance` error if `offsetstdev` is infinite.
#[must_use]
pub fn wood(
    width: u32,
    height: u32,
    offsetstdev: f64,
    length_scale: f64,
    wood_profile: &WoodProfile,
) -> Result<image::RgbImage, rand_distr::NormalError> {
    use rand::Rng;
    let mut imgbuf = image::RgbImage::new(width, height);

    let noise = Noise::gen_noise(width as usize, height as usize);

    /* algorithm taken and modified from https://lodev.org/cgtutor/randomnoise.html#Wood */
    let turb = 14.6; //makes twists
    let turb_size = 32.0; //initial size of the turbulence

    let mut rng = rand::thread_rng();
    let distr = rand_distr::Normal::new(0., offsetstdev)?;
    let offset_x = rng.sample(distr);
    let offset_y = rng.sample(distr);

    // There is an abs later in the function, so we only need from 0 to pi.
    let phase = rng.sample(Uniform::from(0.0..core::f64::consts::PI));

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let x_value_times_scale = f64::from(x) - f64::from(width) / 2. + offset_x; // dimension: px
        let y_value_times_scale = f64::from(y) - f64::from(height) / 2. + offset_y; // dimension: px
        let dist_value_times_scale = x_value_times_scale.hypot(y_value_times_scale)
            + turb * noise.turbulence(f64::from(x), f64::from(y), turb_size) / 256.0;

        #[allow(clippy::cast_possible_truncation)]
        let sine_value = (dist_value_times_scale / length_scale)
            .mul_add(core::f64::consts::PI, phase)
            .sin()
            .abs()
            .powf(0.4) as f32;
        *pixel = lerp_pixel(
            image::Rgb(wood_profile.dark_color),
            image::Rgb(wood_profile.light_color),
            sine_value,
        );
    }

    Ok(image::imageops::colorops::brighten(
        &imgbuf,
        wood_profile.brightness_adjustment,
    ))
}

use interpolation::Lerp;

fn lerp_pixel(a: image::Rgb<u8>, b: image::Rgb<u8>, t: f32) -> image::Rgb<u8> {
    image::Rgb([
        (a.0[0]).lerp(&b.0[0], &t),
        (a.0[1]).lerp(&b.0[1], &t),
        (a.0[2]).lerp(&b.0[2], &t),
    ])
}
