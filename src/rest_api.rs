use clustervms::{CameraId, StreamId};
use log::warn;
use rocket::http::Status;
use rocket::State;
use std::sync::Arc;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::common::VideoTrackMap;
use crate::webrtc_utils;



#[post("/cameras/<camera_id>/streams/<stream_id>/sdp", data="<sdp>")]
async fn handle_sdp_offer(camera_id: CameraId, stream_id: StreamId, sdp: String, video_tracks_state: &State<VideoTrackMap>) -> (Status, String) {
	match video_tracks_state.inner().get(&camera_id).and_then(|stream_map| stream_map.get(&stream_id)) {
		Some(video_track) => {
			match RTCSessionDescription::offer(sdp) {
				Ok(offer) => {
					match webrtc_utils::create_answer(offer, Arc::clone(video_track)).await {
						Ok(local_desc) => {
							return (Status::Created, local_desc.sdp);
						},
						Err(e) => {
							warn!("Error creating SDP answer: {}", e);
							return (Status::BadRequest, String::from("bad request"));
						}
					}
				},
				Err(e) => {
					warn!("Error parsing SDP offer: {}", e);
					return (Status::BadRequest, String::from("bad request"));
				}
			}
		},
		None => {
			warn!("Could not find track for camera {camera_id}, stream {stream_id}");
			return (Status::BadRequest, String::from("bad request"));
		}
	}
}

#[catch(404)]
fn not_found() -> &'static str {
	"Resource was not found."
}

pub fn stage(video_tracks: VideoTrackMap) -> rocket::fairing::AdHoc {
	rocket::fairing::AdHoc::on_ignite("SDP", |rocket| async {
		rocket
			.manage(video_tracks)
			.register("/", catchers![not_found])
			.mount("/", routes![handle_sdp_offer])
	})
}
