//! Where the Flow Bar sits and when it shows (BAR-04, BAR-05).

use serde::{Deserialize, Serialize};

use crate::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FlowBarSettings {
    /// Show the resting bar; when off it appears only while dictating or
    /// with a notice.
    pub always_show: bool,
    /// The resting bar stays hidden until then (seconds since 1970; Hide for 1 hour).
    pub hidden_until: u64,
    pub edge: Edge,
    /// Where along the edge, from 0 (left or top) to 1.
    pub position: f64,
}

impl Default for FlowBarSettings {
    fn default() -> Self {
        Self {
            always_show: true,
            hidden_until: 0,
            edge: Edge::Bottom,
            position: 0.5,
        }
    }
}

impl Eq for FlowBarSettings {}

impl FlowBarSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.position) {
            return Err(t!("error.settings.flowBarPosition"));
        }
        Ok(())
    }
}
