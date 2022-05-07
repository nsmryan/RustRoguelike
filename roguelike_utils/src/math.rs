
pub fn lerp(first: f32, second: f32, scale: f32) -> f32 {
    return first + ((second - first) * scale);
}

pub fn clamp<N: Ord>(val: N, min: N, max: N) -> N {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
}

pub fn clampf(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
}

