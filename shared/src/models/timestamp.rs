use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use yew::{Html, ToHtml, html};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Timestamp(pub DateTime<Utc>);

impl ToHtml for Timestamp {
    fn to_html(&self) -> Html {
        html! { self.0.format("%Y-%m-%d %H:%M:%S").to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json;

    #[test]
    fn test_timestamp_formatting() {
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let timestamp = Timestamp(dt);
        let html_output = timestamp.to_html();

        assert_eq!(html_output, html! { "2025-03-08 14:30:00" });
    }

    #[test]
    fn test_timestamp_serialization() {
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let timestamp = Timestamp(dt);
        let serialized = serde_json::to_string(&timestamp).unwrap();

        assert_eq!(serialized, "\"2025-03-08T14:30:00Z\"");
    }

    #[test]
    fn test_timestamp_deserialization() {
        let json_str = "\"2025-03-08T14:30:00Z\"";
        let deserialized: Timestamp = serde_json::from_str(json_str).unwrap();

        let expected_dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        assert_eq!(deserialized.0, expected_dt);
    }

    #[test]
    fn test_timestamp_equality() {
        let dt1 = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let dt2 = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 1).unwrap();

        let ts1 = Timestamp(dt1);
        let ts2 = Timestamp(dt1);
        let ts3 = Timestamp(dt2);

        assert_eq!(ts1, ts2); // Same timestamp should be equal
        assert_ne!(ts1, ts3); // Different timestamp should not be equal
    }
}
