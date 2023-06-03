use std::collections::HashMap;

use serde_json::{Map, Value};
use statrs::statistics::Data;
use statrs::statistics::OrderStatistics;

use crate::indicators::{fraction, mediant, set_meta, set_result, Calculate, Indicators, Settings};

macro_rules! flag {
    ( $self:ident , $item:ident , $field:ident , $group:ident ) => {
        let ratios: HashMap<String, f64> = std::mem::take(&mut $item.$field)
            .into_iter()
            .filter_map(|(id, fraction)| {
                if fraction.denominator >= $self.minimum_submitted_bids {
                    Some((id, fraction.into()))
                } else {
                    None
                }
            })
            .collect();

        let upper_fence = $self.threshold.unwrap_or_else(|| {
            let mut data = Data::new(ratios.values().copied().collect::<Vec<_>>());
            let q1 = data.lower_quartile();
            let q3 = data.upper_quartile();
            set_meta!($item, R038, format!("{:?}_q1", crate::indicators::Group::$group), q1);
            set_meta!($item, R038, format!("{:?}_q3", crate::indicators::Group::$group), q3);
            // q3 + IQR * 1.5
            (q3 - q1).mul_add(1.5, q3)
        });

        set_meta!(
            $item,
            R038,
            format!("{:?}_upper_fence", crate::indicators::Group::$group),
            upper_fence
        );

        for (id, ratio) in ratios {
            if ratio >= upper_fence {
                set_result!($item, $group, id, R038, ratio);
            }
        }
    };
}

#[derive(Default)]
pub struct R038 {
    threshold: Option<f64>, // resolved in reduce()
    minimum_submitted_bids: usize,
}

impl Calculate for R038 {
    fn new(settings: &mut Settings) -> Self {
        let setting = std::mem::take(&mut settings.R038).unwrap_or_default();

        Self {
            threshold: setting.threshold,
            minimum_submitted_bids: setting.minimum_submitted_bids.unwrap_or(1),
        }
    }

    fn fold(&self, item: &mut Indicators, release: &Map<String, Value>, _ocid: &str) {
        let submitted_bids = Indicators::get_submitted_bids(release);

        // Avoid NaN errors.
        let submitted_bids_count = submitted_bids.len();
        if submitted_bids_count == 0 {
            return;
        }

        let mut disqualified_bids_count = 0;

        for bid in submitted_bids {
            let increment = if let Some(Value::String(status)) = bid.get("status")
                && status == "disqualified"
            {
                1
            } else {
                0
            };

            disqualified_bids_count += increment;

            if let Some(Value::Array(tenderers)) = bid.get("tenderers") {
                for tenderer in tenderers {
                    if let Some(Value::String(id)) = tenderer.get("id") {
                        let fraction = item.r038_tenderer.entry(id.clone()).or_default();
                        *fraction += fraction!(increment, 1);
                    }
                }
            }
        }

        if let Some(Value::Object(buyer)) = release.get("buyer")
            && let Some(Value::String(id)) = buyer.get("id")
        {
            let fraction = item.r038_buyer.entry(id.clone()).or_default();
            *fraction += fraction!(disqualified_bids_count, submitted_bids_count);
        }

        if let Some(Value::Object(tender)) = release.get("tender")
            && let Some(Value::Object(procuring_entity)) = tender.get("procuringEntity")
            && let Some(Value::String(id)) = procuring_entity.get("id")
        {
            let fraction = item.r038_procuring_entity.entry(id.clone()).or_default();
            *fraction += fraction!(disqualified_bids_count, submitted_bids_count);
        }
    }

    fn reduce(&self, item: &mut Indicators, other: &mut Indicators) {
        mediant!(item, other, r038_buyer);
        mediant!(item, other, r038_procuring_entity);
        mediant!(item, other, r038_tenderer);
    }

    fn finalize(&self, item: &mut Indicators) {
        flag!(self, item, r038_buyer, Buyer);
        flag!(self, item, r038_procuring_entity, ProcuringEntity);
        flag!(self, item, r038_tenderer, Tenderer);
    }
}
