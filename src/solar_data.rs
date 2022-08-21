#[derive(Debug, Default)]
pub struct SolarData {
    pub id: u64,
    pub device_id: u8,
    pub tracker_id: u8,
    pub timestamp: u64,
    pub energy_generation: usize,
    pub power_generation: usize,
    pub temperature: f32,
    pub voltage: f32,
    pub power_generation_v7: f32,
    pub power_generation_v8: f32,
    pub power_generation_v9: f32,
}
