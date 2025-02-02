use std::prelude::v1::*;

use std::{fmt, str::FromStr};

use chrono::prelude::*;
use serde::{
    de::{self, Deserializer, MapAccess, Visitor},
    Deserialize,
};

mod app_keys_list;
pub use app_keys_list::AppKeysList;

mod metrics_hierarchy;
pub use metrics_hierarchy::MetricsHierarchy;

mod systemtime {
    use chrono::DateTime;

    #[repr(transparent)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PeriodTime(pub i64);

    impl<Tz: chrono::TimeZone> From<DateTime<Tz>> for PeriodTime {
        fn from(dt: DateTime<Tz>) -> PeriodTime {
            PeriodTime(dt.timestamp())
        }
    }
}

pub use systemtime::PeriodTime;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename = "usage_report")]
pub struct UsageReport {
    pub metric: String,
    pub period: Period,
    pub period_start: PeriodTime,
    pub period_end: PeriodTime,
    pub max_value: u64,
    pub current_value: u64,
}

// Unfortunately the XML output from Apisonator includes a rather useless "usage_reports" tag that
// is then followed by a "usage_report" tag in each UsageReport, so we need to wrap that up.
#[cfg_attr(supports_transparent_enums, repr(transparent))]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum UsageReports {
    #[serde(rename = "usage_report")]
    UsageReports(Vec<UsageReport>),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Authorization {
    Status(AuthorizationStatus),
    Error(AuthorizationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthorizationStatus {
    authorized: bool,
    reason: Option<String>,
    plan: String,
    usage_reports: Option<UsageReports>,

    #[serde(rename = "hierarchy")]
    metrics_hierarchy: Option<MetricsHierarchy>,

    #[serde(rename = "app_keys")]
    app_keys: Option<AppKeysList>,
}

impl AuthorizationStatus {
    pub fn authorized(&self) -> bool {
        self.authorized
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn plan(&self) -> &str {
        self.plan.as_ref()
    }

    pub fn app_keys(&self) -> Option<&AppKeysList> {
        self.app_keys.as_ref()
    }

    pub fn usage_reports(&self) -> Option<&UsageReports> {
        self.usage_reports.as_ref()
    }

    pub fn hierarchy(&self) -> Option<&MetricsHierarchy> {
        self.metrics_hierarchy.as_ref()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthorizationError {
    code: String,
}

impl AuthorizationError {
    pub fn code(&self) -> &str {
        self.code.as_ref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Period {
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
    Eternity,
}

struct PeriodStringVisitor;

impl<'de> Visitor<'de> for PeriodStringVisitor {
    type Value = Period;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string that represents a period")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "minute" => Ok(Period::Minute),
            "hour" => Ok(Period::Hour),
            "day" => Ok(Period::Day),
            "week" => Ok(Period::Week),
            "month" => Ok(Period::Month),
            "year" => Ok(Period::Year),
            "eternity" => Ok(Period::Eternity),
            _ => Err(E::custom("Invalid period")),
        }
    }
}

impl<'de> Deserialize<'de> for Period {
    fn deserialize<D>(deserializer: D) -> Result<Period, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PeriodStringVisitor)
    }
}

struct TimestampVisitor;

impl<'de> Visitor<'de> for TimestampVisitor {
    type Value = PeriodTime;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string that represents a timestamp")
    }

    fn visit_map<V>(self, mut map: V) -> Result<PeriodTime, V::Error>
    where
        V: MapAccess<'de>,
    {
        // We know there's only one key with one value.
        // The key is not used, but we need to call "next_key()". From the
        // docs: "Calling `next_value` before `next_key` is incorrect and is
        // allowed to panic or return bogus results.".
        let _key: Option<String> = map.next_key()?;
        let timestamp: String = map.next_value()?;

        let ts_str = timestamp.as_str();
        let dt = DateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S %z").map_err(|e| {
            de::Error::custom(format_args!(
                "invalid timestamp {}, expected %Y-%m-%d %H:%M:%S %z: {:?}",
                ts_str, e
            ))
        })?;

        Ok(PeriodTime::from(dt))
    }
}

impl<'de> Deserialize<'de> for PeriodTime {
    fn deserialize<D>(deserializer: D) -> Result<PeriodTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TimestampVisitor)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct UsageData {
    max_value: u64,
    current_value: u64,
}

#[repr(transparent)]
#[derive(Debug, Deserialize, PartialEq)]
pub struct Metric(pub String);

impl FromStr for Authorization {
    type Err = serde_xml_rs::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_xml_rs::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::{UsageReports::*, *};

    #[test]
    fn parse() {
        let s = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>App Plan</plan>
            <usage_reports>
                <usage_report metric="products" period="minute">
                    <period_start>2019-06-05 16:24:00 +0000</period_start>
                    <period_end>2019-06-05 16:25:00 +0000</period_end>
                    <max_value>5</max_value>
                    <current_value>0</current_value>
                </usage_report>
                <usage_report metric="products" period="month">
                    <period_start>2019-06-01 00:00:00 +0000</period_start>
                    <period_end>2019-07-01 00:00:00 +0000</period_end>
                    <max_value>50</max_value>
                    <current_value>0</current_value>
                </usage_report>
            </usage_reports>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(s).unwrap();

        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: true,
            reason: None,
            plan: String::from("App Plan"),
            metrics_hierarchy: None,
            app_keys: None,
            usage_reports: Some(UsageReports(vec![
                UsageReport {
                    metric: String::from("products"),
                    period: Period::Minute,
                    period_start: Utc.ymd(2019, 6, 5).and_hms(16, 24, 0).into(),
                    period_end: Utc.ymd(2019, 6, 5).and_hms(16, 25, 0).into(),
                    max_value: 5,
                    current_value: 0,
                },
                UsageReport {
                    metric: String::from("products"),
                    period: Period::Month,
                    period_start: Utc.ymd(2019, 6, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2019, 7, 1).and_hms(0, 0, 0).into(),
                    max_value: 50,
                    current_value: 0,
                },
            ])),
        });

        assert_eq!(parsed_auth, expected_auth);
    }

    #[test]
    fn parse_invalid_date_format() {
        let s = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>App Plan</plan>
            <usage_reports>
                <usage_report metric="products" period="minute">
                    <period_start>05-06-2019 16:24:00 +0000</period_start>
                    <period_end>05-06-2019 16:25:00 +0000</period_end>
                    <max_value>5</max_value>
                    <current_value>0</current_value>
                </usage_report>
                <usage_report metric="products" period="month">
                    <period_start>2019-06-01 00:00:00 +0000</period_start>
                    <period_end>2019-07-01 00:00:00 +0000</period_end>
                    <max_value>50</max_value>
                    <current_value>0</current_value>
                </usage_report>
            </usage_reports>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(s);

        assert!(parsed_auth.is_err());

        let s = format!("{}", parsed_auth.unwrap_err());
        assert!(s.contains("invalid timestamp"));
    }

    #[test]
    fn parse_response_with_no_usage_reports() {
        let s = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>App Plan</plan>
        </status>
        "##;
        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: true,
            reason: None,
            plan: "App Plan".into(),
            usage_reports: None,
            metrics_hierarchy: None,
            app_keys: None,
        });
        let parsed_auth = Authorization::from_str(s)
            .expect("failed to parse authorization without usage reports");

        assert_eq!(expected_auth, parsed_auth);
    }

    #[test]
    fn parse_error_authorization() {
        let xml_response = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <error code="user_key_invalid">user key "some_user_key" is invalid</error>
        "##;

        let parsed_auth = Authorization::from_str(xml_response).unwrap();

        let expected_auth = Authorization::Error(AuthorizationError {
            code: String::from("user_key_invalid"),
        });
        assert_eq!(parsed_auth, expected_auth);
    }

    #[test]
    fn parse_denied_authorization() {
        let xml_response = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
          <authorized>false</authorized>
          <reason>application key is missing</reason>
          <plan>sample</plan>
          <usage_reports>
            <usage_report metric="ticks" period="minute">
              <period_start>2021-06-08 18:07:00 +0000</period_start>
              <period_end>2021-06-08 18:08:00 +0000</period_end>
              <max_value>5</max_value>
              <current_value>0</current_value>
            </usage_report>
          </usage_reports>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(xml_response).unwrap();

        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: false,
            reason: Some("application key is missing".into()),
            plan: "sample".into(),
            usage_reports: Some(UsageReports(vec![UsageReport {
                metric: String::from("ticks"),
                period: Period::Minute,
                period_start: Utc.ymd(2021, 6, 8).and_hms(18, 7, 0).into(),
                period_end: Utc.ymd(2021, 6, 8).and_hms(18, 8, 0).into(),
                max_value: 5,
                current_value: 0,
            }])),
            metrics_hierarchy: None,
            app_keys: None,
        });
        assert_eq!(expected_auth, parsed_auth);
    }

    #[test]
    fn parse_metrics_hierarchy() {
        let xml_response = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>Basic</plan>
            <usage_reports>
                <usage_report metric="parent1" period="day">
                    <period_start>2016-01-01 00:00:00 +0000</period_start>
                    <period_end>2016-01-02 00:00:00 +0000</period_end>
                    <max_value>100</max_value>
                    <current_value>20</current_value>
                </usage_report>
                <usage_report metric="parent2" period="day">
                    <period_start>2016-01-01 00:00:00 +0000</period_start>
                    <period_end>2016-01-02 00:00:00 +0000</period_end>
                    <max_value>100</max_value>
                    <current_value>10</current_value>
                </usage_report>
                <usage_report metric="child1" period="day">
                    <period_start>2016-01-01 00:00:00 +0000</period_start>
                    <period_end>2016-01-02 00:00:00 +0000</period_end>
                    <max_value>100</max_value>
                    <current_value>10</current_value>
                </usage_report>
                <usage_report metric="child2" period="day">
                    <period_start>2016-01-01 00:00:00 +0000</period_start>
                    <period_end>2016-01-02 00:00:00 +0000</period_end>
                    <max_value>100</max_value>
                    <current_value>10</current_value>
                </usage_report>
                <usage_report metric="child3" period="day">
                    <period_start>2016-01-01 00:00:00 +0000</period_start>
                    <period_end>2016-01-02 00:00:00 +0000</period_end>
                    <max_value>100</max_value>
                    <current_value>10</current_value>
                </usage_report>
            </usage_reports>
            <hierarchy>
                <metric name="parent1" children="child1 child2" />
                <metric name="parent2" children="child3" />
            </hierarchy>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(xml_response).unwrap();

        let mut expected_hierarchy = MetricsHierarchy::new();
        expected_hierarchy.insert(
            "parent1",
            vec![String::from("child1"), String::from("child2")],
        );
        expected_hierarchy.insert("parent2", vec![String::from("child3")]);

        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: true,
            reason: None,
            plan: String::from("Basic"),
            metrics_hierarchy: Some(expected_hierarchy),
            app_keys: None,
            usage_reports: Some(UsageReports(vec![
                UsageReport {
                    metric: String::from("parent1"),
                    period: Period::Day,
                    period_start: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0).into(),
                    max_value: 100,
                    current_value: 20,
                },
                UsageReport {
                    metric: String::from("parent2"),
                    period: Period::Day,
                    period_start: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0).into(),
                    max_value: 100,
                    current_value: 10,
                },
                UsageReport {
                    metric: String::from("child1"),
                    period: Period::Day,
                    period_start: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0).into(),
                    max_value: 100,
                    current_value: 10,
                },
                UsageReport {
                    metric: String::from("child2"),
                    period: Period::Day,
                    period_start: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0).into(),
                    max_value: 100,
                    current_value: 10,
                },
                UsageReport {
                    metric: String::from("child3"),
                    period: Period::Day,
                    period_start: Utc.ymd(2016, 1, 1).and_hms(0, 0, 0).into(),
                    period_end: Utc.ymd(2016, 1, 2).and_hms(0, 0, 0).into(),
                    max_value: 100,
                    current_value: 10,
                },
            ])),
        });

        assert_eq!(parsed_auth, expected_auth);
    }

    #[test]
    fn metrics_hierarchy_remove() {
        let mut hierarchy = MetricsHierarchy::new();

        hierarchy.insert(
            "parent1",
            vec![String::from("child1"), String::from("child2")],
        );
        hierarchy.insert("parent2", vec![String::from("child3")]);

        hierarchy.remove("parent1");

        let mut expected_hierarchy = MetricsHierarchy::new();

        expected_hierarchy.insert("parent2", vec![String::from("child3")]);

        assert_eq!(hierarchy, expected_hierarchy);
    }

    #[test]
    fn metrics_hierarchy_parent_of() {
        let a_parent = "a_parent";
        let mut hierarchy = MetricsHierarchy::new();

        hierarchy.insert(
            a_parent,
            vec![String::from("child1"), String::from("child2")],
        );
        hierarchy.insert("parent2", vec![String::from("child3")]);

        assert_eq!(hierarchy.parent_of("child2"), Some(a_parent));
        assert_eq!(hierarchy.parent_of("child3"), Some("parent2"));
        assert_eq!(hierarchy.parent_of("nonchild"), None);
        assert_eq!(hierarchy.parent_of(a_parent), None);
    }

    #[test]
    fn parse_app_keys() {
        let xml_response = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>Basic</plan>
            <app_keys app="app_id" svc="service_id">
                <key id="a_secret_key" />
                <key id="another_secret_key"/>
            </app_keys>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(xml_response).unwrap();

        let expected_app_keys = AppKeysList::new(
            "service_id".into(),
            "app_id".into(),
            vec!["a_secret_key", "another_secret_key"],
        );

        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: true,
            reason: None,
            plan: String::from("Basic"),
            app_keys: Some(expected_app_keys),
            metrics_hierarchy: None,
            usage_reports: None,
        });

        assert_eq!(parsed_auth, expected_auth);
    }

    #[test]
    fn parse_empty_app_keys() {
        let xml_response = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <status>
            <authorized>true</authorized>
            <plan>Basic</plan>
            <app_keys app="app_id" svc="service_id">
            </app_keys>
        </status>
        "##;

        let parsed_auth = Authorization::from_str(xml_response).unwrap();

        let expected_app_keys = AppKeysList::new(
            "service_id".into(),
            "app_id".into(),
            core::iter::empty::<crate::application::AppKey>(),
        );

        let expected_auth = Authorization::Status(AuthorizationStatus {
            authorized: true,
            reason: None,
            plan: String::from("Basic"),
            app_keys: Some(expected_app_keys),
            metrics_hierarchy: None,
            usage_reports: None,
        });

        assert_eq!(parsed_auth, expected_auth);
    }
}
