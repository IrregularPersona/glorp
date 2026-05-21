#[derive(Default, Clone)]
pub struct FpsStats {
    pub current: f32,
    pub average: f32,
    pub low1: f32,
    pub low01: f32,
    pub history: Vec<f32>,
}