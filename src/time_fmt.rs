use chrono::Utc;

use crate::models::entry::Timestamp;

pub fn format_created_at(ts: &Timestamp) -> String {
    let created = ts.to_naive();
    let now = Utc::now().naive_utc();
    let diff = now.signed_duration_since(created);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_hours() < 1 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        created.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::entry::{Timestamp, TIMESTAMP_FORMAT};
    use chrono::Utc;

    fn ts_ago(secs: i64) -> Timestamp {
        let s = (Utc::now().naive_utc() - chrono::Duration::seconds(secs))
            .format(TIMESTAMP_FORMAT)
            .to_string();
        Timestamp::from_raw(s)
    }

    #[test]
    fn test_just_now() {
        assert_eq!(format_created_at(&ts_ago(5)), "just now");
        assert_eq!(format_created_at(&ts_ago(59)), "just now");
    }

    #[test]
    fn test_minutes_ago() {
        assert_eq!(format_created_at(&ts_ago(60)), "1m ago");
        assert_eq!(format_created_at(&ts_ago(300)), "5m ago");
        assert_eq!(format_created_at(&ts_ago(3599)), "59m ago");
    }

    #[test]
    fn test_hours_ago() {
        assert_eq!(format_created_at(&ts_ago(3600)), "1h ago");
        assert_eq!(format_created_at(&ts_ago(7200)), "2h ago");
    }

    #[test]
    fn test_days_ago() {
        assert_eq!(format_created_at(&ts_ago(86400)), "1d ago");
        assert_eq!(format_created_at(&ts_ago(86400 * 6)), "6d ago");
    }

    #[test]
    fn test_old_shows_date() {
        let result = format_created_at(&ts_ago(86400 * 30));
        assert!(result.starts_with("20"), "expected date, got: {result}");
        assert_eq!(result.len(), 10); // "YYYY-MM-DD"
    }
}
