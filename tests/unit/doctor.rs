use std::time::{Duration, SystemTime};

use super::{age_seconds, age_text, state_age};

#[test]
fn an_age_of_fifty_nine_seconds_reads_just_now() {
    assert_eq!(age_text(59), "just now");
}

#[test]
fn an_age_of_sixty_seconds_reads_one_minute() {
    assert_eq!(age_text(60), "1m");
}

#[test]
fn an_age_of_one_hour_reads_one_hour() {
    assert_eq!(age_text(3600), "1h");
}

#[test]
fn an_age_of_fifty_nine_minutes_still_reads_minutes() {
    assert_eq!(age_text(3599), "59m");
}

#[test]
fn an_age_of_one_hour_and_a_minute_keeps_both_units() {
    assert_eq!(age_text(3660), "1h 1m");
}

#[test]
fn an_age_of_one_day_reads_one_day() {
    assert_eq!(age_text(86_400), "1d");
}

#[test]
fn an_age_of_a_day_and_an_hour_keeps_both_units() {
    assert_eq!(age_text(90_000), "1d 1h");
}

#[test]
fn a_state_directory_has_no_age() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state.json");
    std::fs::create_dir(&state).unwrap();

    assert_eq!(state_age(&state).unwrap(), None);
}

#[test]
fn an_age_from_a_future_modification_time_reads_as_zero() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let elapsed = age_seconds(now + Duration::from_secs(60), now);

    assert_eq!(elapsed, 0);
    assert_eq!(age_text(elapsed), "just now");
}
