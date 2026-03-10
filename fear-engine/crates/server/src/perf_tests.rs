//! Performance and load tests for the Fear Engine server.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use chrono::Utc;
    use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
    use fear_engine_fear_profile::analyzer::{BehaviorBaseline, BehaviorFeatures};
    use fear_engine_fear_profile::profile::FearProfile;
    use fear_engine_storage::Database;

    use crate::app::AppState;
    use crate::game_loop::SessionGameLoop;

    #[test]
    fn bench_fear_profile_update_under_10ms() {
        let mut profile = FearProfile::new();
        let features = BehaviorFeatures {
            hesitation_score: 0.7,
            anxiety_score: 0.6,
            avoidance_score: 0.5,
            engagement_score: 0.2,
            indecision_score: 0.4,
            fight_or_flight_ratio: 0.3,
        };

        let start = Instant::now();
        for _ in 0..100 {
            let _ = profile.update(&features);
        }
        let elapsed = start.elapsed();
        let per_update = elapsed / 100;
        assert!(
            per_update.as_millis() < 10,
            "fear profile update took {:?} per call",
            per_update
        );
    }

    #[test]
    fn bench_behavior_batch_processing() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        db.create_fear_profile(&sid).unwrap();
        let mut gl = SessionGameLoop::new(sid, db);
        gl.start_game();

        let events: Vec<BehaviorEvent> = (0..50)
            .map(|i| BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 5.0 + (i % 10) as f64 * 0.2,
                    backspace_count: i % 3,
                    total_chars: 20,
                },
                timestamp: Utc::now(),
                scene_id: "cal_awakening".into(),
            })
            .collect();

        let start = Instant::now();
        gl.process_behavior(events);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "batch processing took {:?}",
            elapsed
        );
    }

    #[test]
    fn bench_scene_graph_traversal() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        db.create_fear_profile(&sid).unwrap();
        let mut gl = SessionGameLoop::new(sid, db);
        gl.start_game();

        let start = Instant::now();
        for _ in 0..100 {
            let _ = gl.process_choice("sit_up", 1500, fear_engine_common::types::ChoiceApproach::Investigate);
            // Reset to awakening for repeated traversal.
        }
        let elapsed = start.elapsed();
        let per_choice = elapsed / 100;
        assert!(
            per_choice.as_millis() < 50,
            "scene traversal took {:?} per choice",
            per_choice
        );
    }

    #[test]
    fn bench_feature_extraction() {
        let events: Vec<BehaviorEvent> = (0..100)
            .map(|i| BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 4.0 + (i % 5) as f64,
                    backspace_count: i % 4,
                    total_chars: 30,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            })
            .collect();
        let baseline = BehaviorBaseline::default();

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = BehaviorFeatures::extract(&events, &baseline);
        }
        let elapsed = start.elapsed();
        let per_extract = elapsed / 1000;
        assert!(
            per_extract.as_micros() < 1000,
            "feature extraction took {:?} per call",
            per_extract
        );
    }

    #[tokio::test]
    async fn test_50_concurrent_websocket_connections() {
        use crate::app::build_app;
        use futures_util::StreamExt;
        use tokio::net::TcpListener;
        use tokio_tungstenite::tungstenite;
        use tower_http::cors::CorsLayer;

        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let app = build_app(state.clone(), CorsLayer::permissive());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut handles = Vec::new();
        for _ in 0..50 {
            let url = format!("ws://{addr}/ws");
            handles.push(tokio::spawn(async move {
                let result = tokio_tungstenite::connect_async(&url).await;
                match result {
                    Ok((mut ws, _)) => {
                        // Read welcome message.
                        let _ = ws.next().await;
                        // Graceful close.
                        let _ = ws
                            .close(Some(tungstenite::protocol::CloseFrame {
                                code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                                reason: "done".into(),
                            }))
                            .await;
                        true
                    }
                    Err(_) => false,
                }
            }));
        }

        let mut successes = 0;
        for handle in handles {
            if handle.await.unwrap_or(false) {
                successes += 1;
            }
        }
        assert!(
            successes >= 45,
            "only {successes}/50 connections succeeded"
        );
    }
}
