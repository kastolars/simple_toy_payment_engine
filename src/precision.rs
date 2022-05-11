pub fn convert_precision(x: f32) -> f32 {
    return ((x * 10000.0).round()) / 10000.0;
}
