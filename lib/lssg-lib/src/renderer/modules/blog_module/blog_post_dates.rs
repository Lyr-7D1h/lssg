use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use log::warn;

use crate::{lssg_error::LssgError, sitetree::Input};

use super::BlogPostOptions;

#[derive(Debug, Clone)]
pub struct BlogPostDates {
    pub modified_on: Option<DateTime<Utc>>,
    pub created_on: Option<DateTime<Utc>>,
}
impl Default for BlogPostDates {
    fn default() -> Self {
        Self {
            modified_on: None,
            created_on: None,
        }
    }
}
impl BlogPostDates {
    pub fn from_post_options(
        post_options: &BlogPostOptions,
        input: &Option<Input>,
    ) -> Result<Self, LssgError> {
        let created_on = match post_options
            .created_on
            .as_ref()
            .map(|s| {
                parse_date_string(&s)
                    .inspect_err(|e| {
                        warn!("Failed to parse created on '{s}': {e}");
                    })
                    .ok()
            })
            .flatten()
        {
            Some(date) => Some(date),
            None => match input {
                Some(Input::Local { path }) => Some(path.metadata()?.modified()?.into()),
                _ => None,
            },
        };

        let modified_on = match post_options
            .modified_on
            .as_ref()
            .map(|s| {
                parse_date_string(s)
                    .inspect_err(|e| {
                        warn!("Failed to parse modified on '{s}': {e}");
                    })
                    .ok()
            })
            .flatten()
        {
            Some(date) => Some(date),
            None => match input {
                Some(Input::Local { path }) => Some(path.metadata()?.modified()?.into()),
                _ => None,
            },
        };

        Ok(Self {
            created_on,
            modified_on,
        })
    }

    pub fn to_pretty_string(&self) -> Option<String> {
        if let Some(date) = &self.modified_on {
            return Some(date.format("Updated on %B %d, %Y").to_string());
        }
        if let Some(date) = &self.created_on {
            return Some(date.format("Created on %B %d, %Y").to_string());
        }
        None
    }
}

fn parse_date_string(input: &String) -> Result<DateTime<Utc>, LssgError> {
    // Try RFC 3339 first (includes timezone): "2025-05-08T14:30:00+02:00"
    if let Ok(dt_fixed) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt_fixed.with_timezone(&Utc));
    }

    // Try full datetime without timezone: "2025-05-08T14:30:00"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&naive_dt));
    }

    // Try date-only formats
    for format in ["%Y-%m-%e", "%Y-%m-%d"] {
        if let Ok(naive_date) = NaiveDate::parse_from_str(input, format) {
            // Use modern chrono method for creating time
            let naive_time = chrono::NaiveTime::from_hms_opt(0, 0, 0)
                .ok_or_else(|| LssgError::parse(format!("Date out of range: {input}")))?;
            let naive_dt = naive_date.and_time(naive_time);
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
    }

    // If none match, return an error
    Err(LssgError::parse(format!("Unknown date format: {input}")))
}
