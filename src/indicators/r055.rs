use chrono::{Duration, NaiveDate, Utc};
use log::warn;
use serde_json::{Map, Value};

use crate::indicators::{set_meta, set_result, sum, Calculate, Indicators, Settings};

#[derive(Default)]
pub struct R055 {
    threshold: Option<f64>, // resolved in finalize()
    start_date: NaiveDate,
    end_date: NaiveDate,
    currency: Option<String>,
}

impl Calculate for R055 {
    fn new(settings: &mut Settings) -> Self {
        let setting = std::mem::take(&mut settings.R055).unwrap_or_default();
        let today = Utc::now().date_naive();

        Self {
            threshold: setting.threshold,
            start_date: setting.start_date.unwrap_or(today - Duration::days(365)),
            end_date: setting.end_date.unwrap_or(today),
            currency: settings.currency.clone(),
        }
    }

    fn fold(&self, item: &mut Indicators, release: &Map<String, Value>, ocid: &str) {
        if let Some(Value::Object(tender)) = release.get("tender")
            && let Some(Value::Object(tender_period)) = tender.get("tenderPeriod")
            && let Some(Value::String(date)) = tender_period.get("startDate")
            && let Some(Value::String(method)) = tender.get("procurementMethod")
            && method == "open"
            && let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%z")
            && date >= self.start_date
            && date <= self.end_date
            && let Some(Value::Object(value)) = tender.get("value")
            && let Some(Value::Number(amount)) = value.get("amount")
            && let Some(Value::String(currency)) = value.get("currency")
            && let Some(amount) = amount.as_f64()
        {
            if currency
                == item
                    .currency
                    .get_or_insert_with(|| self.currency.as_ref().map_or_else(|| currency.clone(), Clone::clone))
            {
                item.r055_open_tender_amount.insert(ocid.to_owned(), amount);
            } else {
                warn!("{} is not {:?}, skipping.", currency, item.currency);
            }
        }

        if let Some(Value::Array(awards)) = release.get("awards") {
            for award in awards {
                if let Some(Value::String(status)) = award.get("status")
                    && let Some(Value::Array(suppliers)) = award.get("suppliers")
                    && suppliers.len() == 1
                    && let Some(Value::String(supplier_id)) = suppliers[0].get("id")
                    && status == "active"
                    && let Some(Value::String(date)) = award.get("date")
                    && let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%z")
                    && date >= self.start_date
                    && date <= self.end_date
                    && let Some(Value::Object(value)) = award.get("value")
                    && let Some(Value::Number(amount)) = value.get("amount")
                    && let Some(Value::String(currency)) = value.get("currency")
                    && let Some(amount) = amount.as_f64()
                {
                    if currency
                        == item.currency.get_or_insert_with(|| {
                            self.currency.as_ref().map_or_else(|| currency.clone(), Clone::clone)
                        })
                    {
                        if let Some(Value::Object(buyer)) = release.get("buyer")
                            && let Some(Value::String(id)) = buyer.get("id")
                        {
                            item.r055_direct_awarded_amount_supplier_buyer
                                .insert((id.clone(), supplier_id.clone()), amount);
                        }
                        if let Some(Value::Object(tender)) = release.get("tender")
                            && let Some(Value::Object(procuring_entity)) = tender.get("procuringEntity")
                            && let Some(Value::String(id)) = procuring_entity.get("id")
                        {
                            item.r055_direct_awarded_amount_supplier_procuring_entity
                                .insert((id.clone(), supplier_id.clone()), amount);
                        }
                    } else {
                        warn!("{} is not {:?}, skipping.", currency, item.currency);
                    }
                }
            }
        }
    }

    fn reduce(&self, item: &mut Indicators, other: &mut Indicators) {
        let mut min = 0.0;
        for (key, value) in std::mem::take(&mut other.r055_open_tender_amount) {
            if min <= value {
                item.r055_open_tender_amount.insert(key, value);
                min = value;
            }
        }
        sum!(item, other, r055_direct_awarded_amount_supplier_buyer);
        sum!(item, other, r055_direct_awarded_amount_supplier_procuring_entity);
    }

    fn finalize(&self, item: &mut Indicators) {
        let min_amount = self.threshold.map_or_else(
            || {
                std::mem::take(&mut item.r055_open_tender_amount)
                    .values()
                    .copied()
                    .last()
                    .unwrap_or(0.0)
            },
            |v| v,
        );
        set_meta!(item, R055, "lower_open_amount", min_amount);

        for (id, amount) in &item.r055_direct_awarded_amount_supplier_buyer {
            if *amount >= min_amount {
                set_result!(item, Buyer, id.0, R055, *amount);
                set_result!(item, Tenderer, id.1, R055, *amount);
            }
        }
        for (id, amount) in &item.r055_direct_awarded_amount_supplier_procuring_entity {
            if *amount >= min_amount {
                set_result!(item, ProcuringEntity, id.0, R055, *amount);
                set_result!(item, Tenderer, id.1, R055, *amount);
            }
        }
    }
}
