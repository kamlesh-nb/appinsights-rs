use chrono::{DateTime, SecondsFormat, Utc};
use http::Uri;

use crate::{
    context::TelemetryContext,
    contracts::{Base, Data, Envelope, PageViewData},
    telemetry::{ContextTags, Measurements, Properties, Telemetry},
    time::{self, Duration},
    uuid::Uuid,
};

/// Represents generic actions on a page like a button click.
///
/// # Examples
/// ```rust, no_run
/// # use appinsights::TelemetryClient;
/// # let client = TelemetryClient::new("<instrumentation key>".to_string());
/// use appinsights::telemetry::{Telemetry, PageViewTelemetry};
/// use http::Uri;
/// use std::time::Duration;
///
/// // create a telemetry item
/// let mut telemetry = PageViewTelemetry::new(
///     "check github repo page",
///     "https://github.com/dmolokanov/appinsights-rs".parse::<Uri>().unwrap(),
/// );
///
/// // attach custom properties, measurements and context tags
/// telemetry.properties_mut().insert("component".to_string(), "data_processor".to_string());
/// telemetry.tags_mut().insert("os_version".to_string(), "linux x86_64".to_string());
/// telemetry.measurements_mut().insert("body_size".to_string(), 115.0);
///
/// // submit telemetry item to server
/// client.track(telemetry);
/// ```
#[derive(Debug)]
pub struct PageViewTelemetry {
    /// Identifier of a generic action on a page.
    /// It is used to correlate a generic action on a page and telemetry generated by the service.
    id: Option<Uuid>,

    /// Event name.
    name: String,

    /// Request URL with all query string parameters.
    uri: Uri,

    /// Request duration.
    duration: Option<Duration>,

    /// The time stamp when this telemetry was measured.
    timestamp: DateTime<Utc>,

    /// Custom properties.
    properties: Properties,

    /// Telemetry context containing extra, optional tags.
    tags: ContextTags,

    /// Custom measurements.
    measurements: Measurements,
}

impl PageViewTelemetry {
    /// Creates a new page view telemetry item with the specified name and url.
    pub fn new(name: impl Into<String>, uri: Uri) -> Self {
        Self {
            id: Option::default(),
            name: name.into(),
            uri,
            duration: Option::default(),
            timestamp: time::now(),
            properties: Properties::default(),
            tags: ContextTags::default(),
            measurements: Measurements::default(),
        }
    }

    /// Returns custom measurements to submit with the telemetry item.
    pub fn measurements(&self) -> &Measurements {
        &self.measurements
    }

    /// Returns mutable reference to custom measurements.
    pub fn measurements_mut(&mut self) -> &mut Measurements {
        &mut self.measurements
    }
}

impl Telemetry for PageViewTelemetry {
    /// Returns the time when this telemetry was measured.
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Returns custom properties to submit with the telemetry item.
    fn properties(&self) -> &Properties {
        &self.properties
    }

    /// Returns mutable reference to custom properties.
    fn properties_mut(&mut self) -> &mut Properties {
        &mut self.properties
    }

    /// Returns context data containing extra, optional tags. Overrides values found on client telemetry context.
    fn tags(&self) -> &ContextTags {
        &self.tags
    }

    /// Returns mutable reference to custom tags.
    fn tags_mut(&mut self) -> &mut ContextTags {
        &mut self.tags
    }
}

impl From<(TelemetryContext, PageViewTelemetry)> for Envelope {
    fn from((context, telemetry): (TelemetryContext, PageViewTelemetry)) -> Self {
        Self {
            name: "Microsoft.ApplicationInsights.PageView".into(),
            time: telemetry.timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
            i_key: Some(context.i_key),
            tags: Some(ContextTags::combine(context.tags, telemetry.tags).into()),
            data: Some(Base::Data(Data::PageViewData(PageViewData {
                name: telemetry.name,
                url: Some(telemetry.uri.to_string()),
                duration: telemetry.duration.map(|duration| duration.to_string()),
                referrer_uri: None,
                id: telemetry
                    .id
                    .map(|id| id.as_hyphenated().to_string())
                    .unwrap_or_default(),
                properties: Some(Properties::combine(context.properties, telemetry.properties).into()),
                measurements: Some(telemetry.measurements.into()),
                ..PageViewData::default()
            }))),
            ..Envelope::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::TimeZone;

    use super::*;

    #[test]
    fn it_overrides_properties_from_context() {
        time::set(Utc.ymd(2019, 1, 2).and_hms_milli(3, 4, 5, 800));

        let mut context =
            TelemetryContext::new("instrumentation".into(), ContextTags::default(), Properties::default());
        context.properties_mut().insert("test".into(), "ok".into());
        context.properties_mut().insert("no-write".into(), "fail".into());

        let mut telemetry = PageViewTelemetry::new("page updated", "https://example.com/main.html".parse().unwrap());
        telemetry.properties_mut().insert("no-write".into(), "ok".into());
        telemetry.measurements_mut().insert("latency".into(), 200.0);

        let envelop = Envelope::from((context, telemetry));

        let expected = Envelope {
            name: "Microsoft.ApplicationInsights.PageView".into(),
            time: "2019-01-02T03:04:05.800Z".into(),
            i_key: Some("instrumentation".into()),
            tags: Some(BTreeMap::default()),
            data: Some(Base::Data(Data::PageViewData(PageViewData {
                name: "page updated".into(),
                url: Some("https://example.com/main.html".into()),
                properties: Some({
                    let mut properties = BTreeMap::default();
                    properties.insert("test".into(), "ok".into());
                    properties.insert("no-write".into(), "ok".into());
                    properties
                }),
                measurements: Some({
                    let mut measurement = BTreeMap::default();
                    measurement.insert("latency".into(), 200.0);
                    measurement
                }),
                ..PageViewData::default()
            }))),
            ..Envelope::default()
        };

        assert_eq!(envelop, expected)
    }

    #[test]
    fn it_overrides_tags_from_context() {
        time::set(Utc.ymd(2019, 1, 2).and_hms_milli(3, 4, 5, 700));

        let mut context =
            TelemetryContext::new("instrumentation".into(), ContextTags::default(), Properties::default());
        context.tags_mut().insert("test".into(), "ok".into());
        context.tags_mut().insert("no-write".into(), "fail".into());

        let mut telemetry = PageViewTelemetry::new("page updated", "https://example.com/main.html".parse().unwrap());
        telemetry.tags_mut().insert("no-write".into(), "ok".into());

        let envelop = Envelope::from((context, telemetry));

        let expected = Envelope {
            name: "Microsoft.ApplicationInsights.PageView".into(),
            time: "2019-01-02T03:04:05.700Z".into(),
            i_key: Some("instrumentation".into()),
            tags: Some({
                let mut tags = BTreeMap::default();
                tags.insert("test".into(), "ok".into());
                tags.insert("no-write".into(), "ok".into());
                tags
            }),
            data: Some(Base::Data(Data::PageViewData(PageViewData {
                name: "page updated".into(),
                url: Some("https://example.com/main.html".into()),
                properties: Some(BTreeMap::default()),
                measurements: Some(BTreeMap::default()),
                ..PageViewData::default()
            }))),
            ..Envelope::default()
        };

        assert_eq!(envelop, expected)
    }
}
