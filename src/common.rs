use clustervms::{CameraId, StreamId};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

pub type VideoTrackMap = HashMap::<CameraId, HashMap<StreamId, Arc<TrackLocalStaticRTP>>>;

#[derive(serde::Deserialize)]
pub struct StreamSettings {
	pub source_url: Url,
	pub username: String,
	pub password: String,
}
