use log::warn;
use rocket::http::Status;
use rocket::State;
use std::sync::Arc;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

use crate::webrtc_utils;



#[post("/sdp", data="<sdp>")]
async fn handle_sdp_offer(sdp: String, video_track_state: &State<Arc<TrackLocalStaticRTP>>) -> (Status, String) {
	match RTCSessionDescription::offer(sdp) {
		Ok(offer) => {
			match webrtc_utils::create_answer(offer, Arc::clone(video_track_state.inner())).await {
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
}

#[catch(404)]
fn not_found() -> &'static str {
	"Resource was not found."
}

pub fn stage(video_track: Arc<TrackLocalStaticRTP>) -> rocket::fairing::AdHoc {
	rocket::fairing::AdHoc::on_ignite("SDP", |rocket| async {
		rocket
			.manage(video_track)
			.register("/", catchers![not_found])
			.mount("/", routes![handle_sdp_offer])
	})
}
