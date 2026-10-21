use super::*;

#[test]
fn only_explicit_finished_status_advances_the_queue() {
    let mut media = MediaStatus {
        media_session_id: 7,
        player_state: PlayerState::Idle,
        idle_reason: Some(IdleReason::Finished),
        current_time: 12.5,
        duration: Some(20.),
        volume: oxicast::Volume {
            level: 0.3,
            muted: false,
        },
        media: None,
    };
    assert_eq!(status(&media, None).unwrap().state, State::Finished);
    assert_eq!(status(&media, None).unwrap().position_ms, 12500);
    media.idle_reason = Some(IdleReason::Cancelled);
    assert_eq!(status(&media, None).unwrap().state, State::Stopped);
    media.idle_reason = Some(IdleReason::Error);
    assert!(status(&media, None).is_err());
    media.player_state = PlayerState::Playing;
    media.current_time = f64::NAN;
    assert!(status(&media, None).is_err());
}
