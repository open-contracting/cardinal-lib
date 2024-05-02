use std::collections::{HashMap, HashSet};

use chrono::DateTime;
use serde_json::{Map, Value};

use crate::indicators::{set_result, Calculate, Indicators, Settings};
use crate::parse_pipe_separated_value;

#[derive(Default)]
pub struct R003 {
    threshold: i64,
    procurement_methods: HashSet<String>,
    procurement_method_details_thresholds: HashMap<String, i64>,
}

impl R003 {
    fn matches_procurement_method(&self, tender: &Map<String, Value>) -> bool {
        if self.procurement_methods.is_empty() {
            true // match if not filtering out procurement methods
        } else if let Some(Value::String(procurement_method)) = tender.get("procurementMethod") {
            self.procurement_methods.contains(procurement_method)
        } else {
            false
        }
    }
}

impl Calculate for R003 {
    fn new(settings: &mut Settings) -> Self {
        let setting = std::mem::take(&mut settings.R003).unwrap_or_default();

        Self {
            threshold: setting.threshold.unwrap_or(15),
            procurement_methods: parse_pipe_separated_value(setting.procurement_methods.clone()),
            procurement_method_details_thresholds: setting.procurement_method_details_thresholds.unwrap_or_default(),
        }
    }

    fn fold(&self, item: &mut Indicators, release: &Map<String, Value>, ocid: &str) {
        if let Some(Value::Object(tender)) = release.get("tender")
            && self.matches_procurement_method(tender)
            && let Some(Value::Object(tender_period)) = tender.get("tenderPeriod")
            && let Some(Value::String(start_date)) = tender_period.get("startDate")
            && let Some(Value::String(end_date)) = tender_period.get("endDate")
            && let Ok(start_date) = DateTime::parse_from_rfc3339(start_date)
            && let Ok(end_date) = DateTime::parse_from_rfc3339(end_date)
        {
            let duration = (end_date - start_date).num_days();
            if let Some(Value::String(procurement_method_details)) = tender.get("procurementMethodDetails")
                && self
                    .procurement_method_details_thresholds
                    .contains_key(procurement_method_details)
            {
                if duration < self.procurement_method_details_thresholds[procurement_method_details] {
                    set_result!(item, OCID, ocid, R003, 1.0);
                }
            } else if duration < self.threshold {
                set_result!(item, OCID, ocid, R003, 1.0);
            }
        }
    }
}
