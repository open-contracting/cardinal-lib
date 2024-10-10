use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::indicators::{set_result, Calculate, Indicators, Settings};

#[derive(Default)]
pub struct R044 {}

impl R044 {
    fn get_address(party: &Value) -> Option<String> {
        let mut full_address: String = String::new();
        if let Some(Value::Object(address)) = party.get("address")
            && address.get("streetAddress").is_some()
        {
            for field in ["streetAddress", "locality", "region", "postalCode", "countryName"] {
                if let Some(Value::String(value)) = address.get(field) {
                    full_address.push_str(value.trim().to_lowercase().as_str());
                    full_address.push(' ');
                }
            }
            Some(full_address)
        } else {
            None
        }
    }
    fn compare_contact_point(party1: Option<Value>, party2: Option<Value>) -> bool {
        if let Some(Value::Object(contact_point_1)) = party1
            && let Some(Value::Object(contact_point_2)) = party2
        {
            for field in ["name", "email", "telephone", "faxNumber", "url"] {
                if let Some(Value::String(field1)) = contact_point_1.get(field)
                    && let Some(Value::String(field2)) = contact_point_2.get(field)
                {
                    return field1 == field2;
                }
            }
        }
        false
    }
}

impl Calculate for R044 {
    fn new(_settings: &mut Settings) -> Self {
        Self::default()
    }

    fn fold(&self, item: &mut Indicators, release: &Map<String, Value>, ocid: &str) {
        if let Some(Value::Array(parties)) = release.get("parties") {
            let tenderers = parties
                .iter()
                .filter_map(|tenderer| {
                    if let Some(Value::Array(roles)) = tenderer.get("roles")
                        && roles.iter().any(|s| s == "tenderer")
                        && let Some(Value::String(id)) = tenderer.get("id")
                    {
                        Some((id.clone(), (Self::get_address(tenderer), tenderer.get("contactPoint"))))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>();
            for (id_to_compare, details_to_compare) in &tenderers {
                for (id, details) in &tenderers {
                    if id_to_compare != id {
                        let address_match = if let Some(address_to_compare) = details_to_compare.0.as_deref()
                            && let Some(address) = details.0.as_deref()
                            && address_to_compare == address
                        {
                            true
                        } else {
                            false
                        };
                        let contact_point_match =
                            Self::compare_contact_point(details.1.cloned(), details_to_compare.1.cloned());
                        if address_match || contact_point_match {
                            set_result!(item, OCID, ocid, R044, 1.0);
                            set_result!(item, Tenderer, id_to_compare, R044, 1.0);
                        }
                    }
                }
            }
        }
    }
}
