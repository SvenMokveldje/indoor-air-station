#![no_std]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub seq: u32,
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub pressure_hpa: f32,
    pub co2_ppm: Option<u16>,
}
