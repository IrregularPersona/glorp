use std::collections::VecDeque;

#[derive(Default, Clone, serde::Deserialize)]
pub struct FpsStats {
    pub current: f32,
    pub average: f32,
    pub low1: f32,
    pub low01: f32,
    #[serde(default)]
    pub history: VecDeque<f32>,
}