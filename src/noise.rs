use bevy::math::*;

#[inline(always)]
pub fn uhash(x: u32) -> u32 {
    // from https://nullprogram.com/blog/2018/07/31/
    let mut x = x ^ (x >> 16);
    x = x.overflowing_mul(0x7feb352d).0;
    x = x ^ (x >> 15);
    x = x.overflowing_mul(0x846ca68b).0;
    x = x ^ (x >> 16);
    x
}

#[inline(always)]
pub fn uhash2(a: u32, b: u32) -> u32 {
    uhash((a.overflowing_mul(1597334673).0) ^ (b.overflowing_mul(3812015801).0))
}

#[inline(always)]
pub fn hash_noise(coord: UVec2, frame: u32) -> f32 {
    let urnd = uhash2(coord.x, (coord.y << 11) + frame);
    unormf(urnd)
}

#[inline(always)]
pub fn unormf(n: u32) -> f32 {
    n as f32 * (1.0 / 0xffffffffu32 as f32)
}
