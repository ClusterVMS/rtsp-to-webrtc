use figment::{Figment, providers::{Format, Toml}};
use serde::Deserialize;
use std::collections::HashMap;

use crate::common::{Camera, CameraId};



#[derive(Clone, Default, Debug, Deserialize)]
pub struct ClusterVmsConfig {
	pub cameras: HashMap<CameraId, Camera>,
}

pub struct ConfigManager {
	config: ClusterVmsConfig,
}

impl ConfigManager {
	pub fn new() -> Self {
		ConfigManager {
			config: ClusterVmsConfig::default(),
		}
	}

	pub fn read_config(&mut self, filenames: Vec<&str>) -> figment::error::Result<()> {
		let mut figment = Figment::new();
		for filename in filenames {
			figment = figment.merge(Toml::file(filename));
		}
		self.config = figment.extract()?;
		
		Ok(())
	}

	pub fn get_config(&self) -> &ClusterVmsConfig {
		&self.config
	}
}
