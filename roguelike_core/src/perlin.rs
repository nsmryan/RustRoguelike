//! This implementation came from the noisy crate, modified
//! for this game. This crate is in the public domain.
//!
//!
//! An implementation of Improved [Perlin Noise]
//! (https://en.wikipedia.org/wiki/Perlin_noise).
//!
//! Based on a improved perlin noise algorithm for 2D, 3D and 4D in C.
//! Which is based on example code by Ken Perlin at Siggraph 2002.
//! With optimisations by Stefan Gustavson (stegu@itn.liu.se).

use oorandom::Rand32;


#[inline]
pub fn fade(t: f64) -> f64 {
    t * t * t * ( t * ( t * 6.0 - 15.0 ) + 10.0 )
}

#[inline]
pub fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
pub fn if_else(cond: bool, if_true: f64, if_false: f64) -> f64 {
    if cond {
        if_true
    } else {
        if_false
    }
}

pub fn grad1(hash: u8, x: f64) -> f64 {
    let h: u8 = hash & 15;
    let mut grad: f64 = 1.0 + (h & 7) as f64; // Gradient value 1.0, 2.0, ..., 8.0
    if (h & 8) != 0 {
        grad = -grad; // Set a random sign for the gradient
    }

    grad * x // Multiply the gradient with the distance
}

/// Compute 2D gradient-dot-residual vector.
pub fn grad2(hash: u8, x: f64, y: f64) -> f64 {
    // Convert low 3 bits of hash code into 8 simple gradient directions,
    // and compute the dot product with (x,y).
    let h: u8 = hash & 7;
    let u: f64 = if_else(h < 4, x, y);
    let v: f64 = if_else(h < 4, y, x);

    if_else(h & 1 != 0, -u, u) + if_else(h & 2 != 0, -2.0 * v, 2.0 * v)
}

/// Compute 3D gradient-dot-residual vector.
pub fn grad3(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    // Convert low 4 bits of hash code into 12 simple gradient directions,
    // and compute dot product.
    let h: u8 = hash & 15;
    let u: f64 = if_else(h < 8, x, y);
    // Fix repeats at h = 12 to 15
    let v: f64 = if_else(h < 4, y, if_else(h == 12 || h == 14, x, z));

    if_else(h & 1 != 0, -u, u) + if_else(h & 2 != 0, -v, v)
}

#[inline]
pub fn fast_floor(x: f64) -> i64 {
    if x > 0.0 {
        x as i64
    } else {
        (x as i64) - 1
    }
}

/// A Perlin noise generator.
#[derive(Clone, PartialEq, Eq)]
pub struct Perlin {
    perm: Vec<u8>
}

impl Perlin {
    pub fn new(rng: &mut Rand32) -> Perlin {
        let p: Vec<u8> = (0..256).map(|_| (rng.rand_u32() & 0xFF) as u8).collect();
        let perm: Vec<u8> = (0..512).map(|idx:i32| {p[(idx & 255) as usize]}).collect();

        Perlin { perm: perm }
    }

    pub fn noise1d(&self, xin: f64) -> f64 {
        let ix0: i64 = fast_floor(xin); // Integer part of x
        let fx0: f64 = xin - ix0 as f64; // Fractional part of x
        let fx1: f64 = fx0 - 1.0;
        let ix1: i64 = ix0 + 1;

        // Wrap the integer indices at 256, to avoid indexing perm[] out of bounds
        let ii: usize = (ix0 & 255) as usize;
        let jj: usize = (ix1 & 255) as usize;

        // Compute the fade curve.
        let s: f64 = fade(fx0);

        // Work out the hashed gradient indices.
        let gi0: u8 = self.perm[ii] as u8;
        let gi1: u8 = self.perm[jj] as u8;

        // Calculate the gradients.
        let nx0 = grad1(gi0, fx0);
        let nx1 = grad1(gi1, fx1);

        // The result is scaled to return values in the interval [-1, 1].
        0.188 * lerp(s, nx0, nx1)
    }

    pub fn noise2d(&self, xin: f64, yin: f64) -> f64 {
        let ix0: i64 = fast_floor(xin); // Integer part of x
        let iy0: i64 = fast_floor(yin); // Integer part of y
        let fx0: f64 = xin - ix0 as f64; // Fractional part of x
        let fy0: f64 = yin - iy0 as f64; // Fractional part of y
        let fx1: f64 = fx0 - 1.0;
        let fy1: f64 = fy0 - 1.0;

        // Wrap the integer indices at 256, to avoid indexing perm[] out of bounds
        let ix1: usize = ((ix0 + 1) & 255) as usize;
        let iy1: usize = ((iy0 + 1) & 255) as usize;
        let ii: usize = (ix0 & 255) as usize;
        let jj: usize = (iy0 & 255) as usize;

        // Compute the fade curves.
        let t: f64 = fade(fy0);
        let s: f64 = fade(fx0);

        // Work out the hashed gradient indices.
        let gi0: u8 = self.perm[ii + (self.perm[jj] as usize)] as u8;
        let gi1: u8 = self.perm[ii + (self.perm[iy1] as usize)] as u8;
        let gi2: u8 = self.perm[ix1 + (self.perm[jj] as usize)] as u8;
        let gi3: u8 = self.perm[ix1 + (self.perm[iy1] as usize)] as u8;

        // Calculate the gradients.
        let nx0: f64 = grad2(gi0, fx0, fy0);
        let nx1: f64 = grad2(gi1, fx0, fy1);
        let nx2: f64 = grad2(gi2, fx1, fy0);
        let nx3: f64 = grad2(gi3, fx1, fy1);

        let n0: f64 = lerp(t, nx0, nx1);
        let n1: f64 = lerp(t, nx2, nx3);

        // The result is scaled to return values in the interval [-1, 1].
        0.507 * lerp(s, n0, n1)
    }

    pub fn noise3d(&self, xin: f64, yin: f64, zin: f64) -> f64 {
        let ix0: i64 = fast_floor(xin); // Integer part of x
        let iy0: i64 = fast_floor(yin); // Integer part of y
        let iz0: i64 = fast_floor(zin); // Integer part of z
        let fx0: f64 = xin - ix0 as f64; // Fractional part of x
        let fy0: f64 = yin - iy0 as f64; // Fractional part of y
        let fz0: f64 = zin - iz0 as f64; // Fractional part of z
        let fx1: f64 = fx0 - 1.0;
        let fy1: f64 = fy0 - 1.0;
        let fz1: f64 = fz0 - 1.0;

        // Wrap the integer indices at 256, to avoid indexing perm[] out of bounds
        let ix1: usize = ((ix0 + 1) & 255) as usize;
        let iy1: usize = ((iy0 + 1) & 255) as usize;
        let iz1: usize = ((iz0 + 1) & 255) as usize;
        let ii: usize = (ix0 & 255) as usize;
        let jj: usize = (iy0 & 255) as usize;
        let kk: usize = (iz0 & 255) as usize;

        // Compute the fade curves.
        let r: f64 = fade(fz0);
        let t: f64 = fade(fy0);
        let s: f64 = fade(fx0);

        // Work out the hashed gradient indices.
        let gi0: u8 = self.perm[ii + (self.perm[jj + (self.perm[kk] as usize)] as usize)] as u8;
        let gi1: u8 = self.perm[ii + (self.perm[jj + (self.perm[iz1] as usize)] as usize)] as u8;
        let gi2: u8 = self.perm[ii + (self.perm[iy1 + (self.perm[kk] as usize)] as usize)] as u8;
        let gi3: u8 = self.perm[ii + (self.perm[iy1 + (self.perm[iz1] as usize)] as usize)] as u8;
        let gi4: u8 = self.perm[ix1 + (self.perm[jj + (self.perm[kk] as usize)] as usize)] as u8;
        let gi5: u8 = self.perm[ix1 + (self.perm[jj + (self.perm[iz1] as usize)] as usize)] as u8;
        let gi6: u8 = self.perm[ix1 + (self.perm[iy1 + (self.perm[kk] as usize)] as usize)] as u8;
        let gi7: u8 = self.perm[ix1 + (self.perm[iy1 + (self.perm[iz1] as usize)] as usize)] as u8;

        // Calculate the gradients.
        let nxy0: f64 = grad3(gi0, fx0, fy0, fz0);
        let nxy1: f64 = grad3(gi1, fx0, fy0, fz1);
        let nxy2: f64 = grad3(gi2, fx0, fy1, fz0);
        let nxy3: f64 = grad3(gi3, fx0, fy1, fz1);
        let nxy4: f64 = grad3(gi4, fx1, fy0, fz0);
        let nxy5: f64 = grad3(gi5, fx1, fy0, fz1);
        let nxy6: f64 = grad3(gi6, fx1, fy1, fz0);
        let nxy7: f64 = grad3(gi7, fx1, fy1, fz1);

        let nx0: f64 = lerp(r, nxy0, nxy1);
        let nx1: f64 = lerp(r, nxy2, nxy3);
        let nx2: f64 = lerp(r, nxy4, nxy5);
        let nx3: f64 = lerp(r, nxy6, nxy7);

        let n0: f64 = lerp(t, nx0, nx1);
        let n1: f64 = lerp(t, nx2, nx3);

        // The result is scaled to return values in the interval [-1, 1].
        0.936 * lerp(s, n0, n1)
    }
}
