use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

pub type CameraId = u64;
pub type StreamId = u64;

pub type VideoTrackMap = HashMap::<CameraId, HashMap<StreamId, Arc<TrackLocalStaticRTP>>>;

#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Camera {
	pub username: Option<String>,
	pub password: Option<String>,
	pub streams: HashMap<String, Stream>,
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Stream {
	pub source_url: Url,
}
#[derive(serde::Deserialize)]
pub struct StreamSettings {
	pub source_url: Url,
	pub username: String,
	pub password: String,
}

pub fn get_required_env_var(var_name: &str) -> String {
	std::env::var(var_name).expect(format!("Environment variable {var_name} should be set.").as_str())
}
