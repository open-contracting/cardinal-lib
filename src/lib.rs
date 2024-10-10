#![feature(let_chains)]

pub mod indicators;
mod queue;
pub mod standard;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;
use indexmap::IndexMap;
use log::warn;
use rayon::prelude::*;
use serde_json::{Map, Value};

use crate::indicators::is_status;
use crate::indicators::r003::R003;
use crate::indicators::r018::R018;
use crate::indicators::r024::R024;
use crate::indicators::r025::R025;
use crate::indicators::r028::R028;
use crate::indicators::r030::R030;
use crate::indicators::r035::R035;
use crate::indicators::r036::R036;
use crate::indicators::r038::R038;
use crate::indicators::r044::R044;
use crate::indicators::r048::R048;
use crate::indicators::r058::R058;
use crate::indicators::util::{SecondLowestBidRatio, Tenderers};
pub use crate::indicators::{Calculate, Codelist, Exclusions, Group, Indicator, Indicators, Modifications, Settings};
use crate::queue::Job;
use crate::standard::{AWARD_STATUS, BID_STATUS};

macro_rules! add_indicators {
    ( $indicators:ident , $settings:ident , $( $indicator:ident ) ,* , ) => {
        $(
            if $settings.$indicator.is_some() {
                $indicators.push(Box::new($indicator::new(&mut $settings)));
            }
        )*
    }
}

fn parse_pipe_separated_value(value: Option<String>) -> HashSet<String> {
    value
        .unwrap_or_default()
        .split_terminator('|')
        .map(str::to_string)
        .collect()
}

fn parse_pipe_separated_value_with_default(value: Option<String>, default: String) -> HashSet<String> {
    value
        .unwrap_or(default)
        .split_terminator('|')
        .map(str::to_string)
        .collect()
}

///
/// # Errors
///
pub fn init(path: &PathBuf, force: &bool) -> std::io::Result<bool> {
    let content = b"\
; currency = USD
; no_price_comparison_procurement_methods = Random Selection
; price_comparison_procurement_methods = Reverse Auction

; `prepare` command
;
; Read the documentation at:
; https://cardinal.readthedocs.io/en/latest/cli/prepare.html

[defaults]
; currency = USD
; item_classification_scheme = UNSPSC
; bid_status = valid
; award_status = active
; party_roles = true

[redactions]
; amount = 0
; organization_id = placeholder

[corrections]
; award_status_by_contract_status = true

[modifications]
; move_auctions = true
; prefix_buyer_or_procuring_entity_id = DO-UC-
; prefix_tenderer_or_supplier_id = DO-RPE-
; split_procurement_method_details = -

[codelists.bid_status]
; qualified = valid

[codelists.award_status]
; Active = active

; `indicators` command
;
; Read the documentation at:
; https://cardinal.readthedocs.io/en/latest/cli/indicators/

[exclusions]
; procurement_method_details = Random Selection

[R003]
; threshold = 15
; procurement_methods = open|selective|limited

[R003.procurement_method_details]
; emergency = 10
; international = 25

[R018]
; procurement_methods = open|selective

[R024]
; threshold = 0.05

[R025]
; percentile = 75
; threshold = 0.05

[R028]

[R030]

[R035]
; threshold = 1

[R036]

[R038]
; threshold = 0.5
; minimum_submitted_bids = 2
; minimum_contracting_processes = 2

[R044]

[R048]
; digits = 2
; threshold = 10
; minimum_contracting_processes = 20

[R058]
; threshold = 0.5
";

    let stdout = path == &PathBuf::from("-");
    let exists = path.exists();

    if stdout {
        let mut file = io::stdout().lock();
        file.write_all(content)?;
    } else if !exists || *force {
        let mut file = File::create(path)?;
        file.write_all(content)?;
    };

    Ok(stdout)
}

fn fold_reduce<T: Send, Fold, Reduce, Finalize>(
    buffer: impl BufRead + Send,
    default: fn() -> T,
    fold: Fold,
    reduce: Reduce,
    finalize: Finalize,
) -> Result<T>
where
    Fold: Fn(T, Value) -> T + Sync,
    Reduce: Fn(T, T) -> T + Send + Sync,
    Finalize: Fn(T) -> Result<T> + Sync,
{
    let item = buffer
        .lines()
        .enumerate()
        // Other iterators might be faster. par_bridge preserves the index for debugging.
        .par_bridge()
        .fold(default, |mut item, (i, lines_result)| {
            match lines_result {
                Ok(string) => {
                    match serde_json::from_str(&string) {
                        Ok(value) => {
                            item = fold(item, value);
                        }
                        Err(e) => {
                            // Skip empty lines silently.
                            // https://stackoverflow.com/a/64361042/244258
                            if !string.as_bytes().iter().all(u8::is_ascii_whitespace) {
                                warn!("Line {} is invalid JSON, skipping. [{e}]", i + 1);
                            }
                        }
                    }
                }
                // Err: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
                // https://github.com/rust-lang/rust/blob/1.65.0/library/std/src/io/buffered/bufreader.rs#L362-L365
                Err(e) => warn!("Line {} caused an I/O error, skipping. [{e}]", i + 1),
            }
            item
        })
        .reduce(default, reduce);

    finalize(item)
}

impl Indicators {
    pub const fn results(&self) -> &IndexMap<Group, IndexMap<String, HashMap<Indicator, f64>>> {
        &self.results
    }

    fn map() -> Self {
        Self {
            map: true,
            ..Default::default()
        }
    }

    ///
    /// # Errors
    ///
    #[rustfmt::skip]
    pub fn run(buffer: impl BufRead + Send, mut settings: Settings, map: &bool) -> Result<Self> {
        let empty_set: HashSet<String> = HashSet::new();
        let mut indicators: Vec<Box<dyn Calculate + Sync>> = vec![];

        // [exclusions]
        let exclusions = std::mem::take(&mut settings.exclusions).unwrap_or_default();
        let exclude_procurement_method_details = parse_pipe_separated_value(exclusions.procurement_method_details);

        // is_some() must run before indicator initialization, which mutates settings.
        if *map && (settings.R025.is_some() || settings.R038.is_some() || settings.R048.is_some()) {
            indicators.push(Box::new(Tenderers::new(&mut settings)));
        }
        if settings.R024.is_some() || settings.R058.is_some() {
            indicators.push(Box::new(SecondLowestBidRatio::new(&mut settings)));
        }

        if settings.no_price_comparison_procurement_methods.is_some() && settings.price_comparison_procurement_methods.is_some() {
            warn!("no_price_comparison_procurement_methods has no effect if price_comparison_procurement_methods is set.");
        }

        add_indicators!(
            indicators,
            settings,
            R003,
            R018,
            R024,
            R025,
            R028,
            R030,
            R035,
            R036,
            R038,
            R044,
            R048,
            R058,
        );

        let identity = if *map {
            Self::map
        } else {
            Self::default
        };

        fold_reduce(
            buffer,
            identity,
            |mut item, value| {
                if let Value::Object(release) = value
                    && let Some(Value::String(ocid)) = release.get("ocid")
                    && !Self::is_cancelled_contracting_process(&release)
                    && Self::matches_procurement_method_details(&release, &empty_set, &exclude_procurement_method_details)
                {
                    for indicator in &indicators {
                        indicator.fold(&mut item, &release, ocid);
                    }
                }

                item
            },
            |mut item, mut other| {
                let group = item.results.entry(Group::OCID).or_default();
                // If each OCID appears on one line of the file, no overwriting occurs.
                group.extend(std::mem::take(other.results.entry(Group::OCID).or_default()));

                let group = item.results.entry(Group::Tenderer).or_default();
                // The indicator needs to always assign the same value for tenderer results.
                for (key, value) in std::mem::take(other.results.entry(Group::Tenderer).or_default()) {
                    group.entry(key).or_default().extend(value);
                }

                // Note: Buyer and ProcuringEntity indicators are only calculated in finalize().

                for indicator in &indicators {
                    indicator.reduce(&mut item, &mut other);
                }

                item
            },
            |mut item| {
                for indicator in &indicators {
                    indicator.finalize(&mut item);
                }

                // These keys are always set by the reduce closure.
                if item.results[&Group::OCID].is_empty() {
                    item.results.swap_remove(&Group::OCID);
                }
                if item.results[&Group::Tenderer].is_empty() {
                    item.results.swap_remove(&Group::Tenderer);
                }

                // If we return `Ok(item)`, we can't consume temporary internal fields.
                Ok(Self {
                    results: item.results,
                    meta: item.meta,
                    maps: item.maps,
                    ..Default::default()
                })
            },
        )
    }

    // Bids are returned even if there are no awards, because "all" awards are final.
    fn get_complete_awards_and_bids_if_all_awards_final(
        release: &Map<String, Value>,
    ) -> Option<(Vec<&Value>, &Vec<Value>)> {
        if let Some(Value::Array(awards)) = release.get("awards")
            && let Some(Value::Object(bids)) = release.get("bids")
            && let Some(Value::Array(details)) = bids.get("details")
        {
            let mut complete_awards = vec![];

            // An award must be in a final state, in order for indicator results to be stable.
            // Note: OCDS 1.1 uses 'active' to mean "in force". OCDS 1.2 might use 'complete'.
            // https://github.com/open-contracting/standard/issues/1160#issuecomment-1139793598
            for award in awards {
                if let Some(Value::String(status)) = award.get("status") {
                    match status.as_str() {
                        "active" => complete_awards.push(award),
                        "cancelled" | "unsuccessful" => (),
                        _ => return None, // "pending"
                    }
                }
            }

            return Some((complete_awards, details));
        }

        None
    }

    fn is_cancelled_contracting_process(release: &Map<String, Value>) -> bool {
        if let Some(Value::Object(tender)) = release.get("tender")
            && let Some(Value::String(status)) = tender.get("status")
            && status == "cancelled"
        {
            true
        } else {
            false
        }
    }

    fn matches_procurement_method(tender: &Map<String, Value>, set: &HashSet<String>) -> bool {
        if set.is_empty() {
            // Match if not filtering out procurement methods.
            true
        } else if let Some(Value::String(procurement_method)) = tender.get("procurementMethod") {
            set.contains(procurement_method)
        } else {
            // Don't match if filtering out procurement methods.
            false
        }
    }

    fn matches_procurement_method_details(
        release: &Map<String, Value>,
        include: &HashSet<String>,
        exclude: &HashSet<String>,
    ) -> bool {
        if let Some(Value::Object(tender)) = release.get("tender")
            && let Some(Value::String(procurement_method_details)) = tender.get("procurementMethodDetails")
        {
            // A deny list can work even if empty.
            if include.is_empty() {
                !exclude.contains(procurement_method_details)
            // An allow list only works if non-empty.
            } else {
                include.contains(procurement_method_details)
            }
        } else {
            // If the field isn't set, it matches only if using a deny list.
            include.is_empty()
        }
    }

    // Includes pending, valid and disqualified bids.
    fn get_submitted_bids(release: &Map<String, Value>) -> Vec<&Value> {
        let mut submitted_bids = vec![];

        if let Some(Value::Object(bids)) = release.get("bids")
            && let Some(Value::Array(details)) = bids.get("details")
        {
            for bid in details {
                if let Some(Value::String(status)) = bid.get("status")
                    && status != "invited"
                    && status != "withdrawn"
                {
                    submitted_bids.push(bid);
                }
            }
        }

        submitted_bids
    }

    fn get_tenderer_and_value_of_valid_bids(details: &Vec<Value>) -> Vec<(&String, f64, &String)> {
        let mut tuples = vec![];

        for bid in details {
            if let Some(Value::String(status)) = bid.get("status")
                && let Some(Value::Object(value)) = bid.get("value")
                && let Some(Value::Number(amount)) = value.get("amount")
                && let Some(Value::String(currency)) = value.get("currency")
                && let Some(Value::Array(tenderers)) = bid.get("tenderers")
                && tenderers.len() == 1
                && let Some(Value::String(tenderer_id)) = tenderers[0].get("id")
                && let Some(amount) = amount.as_f64()
                && status == "valid"
            {
                tuples.push((tenderer_id, amount, currency));
            }
        }

        tuples
    }
}

macro_rules! stringify {
    ( $object:ident , $key:expr ) => {
        if let Some(Value::Number(id)) = $object.get($key) {
            $object[$key] = Value::String(id.to_string());
        }
    };
}

macro_rules! prepare_id_object {
    ( $field:ident , $key:expr , $redact:ident , $prefix:expr , $lookup:expr , $role:expr ) => {
        if let Some(Value::Object(object)) = $field.get_mut($key) {
            stringify!(object, "id");
            if let Some(Value::String(id)) = object.get_mut("id") {
                if $redact.contains(id) {
                    object.remove("id");
                } else {
                    if !id.starts_with($prefix) {
                        id.insert_str(0, &$prefix);
                    }
                    if let Some(roles) = $lookup.get_mut(id) {
                        roles.insert($role.into());
                    }
                }
            }
        }
    };
}

macro_rules! prepare_id_array {
    ( $field:ident , $key:expr , $redact:ident , $prefix:expr , $lookup:expr , $role:expr ) => {
        // Coerce objects into arrays.
        if let Some(value) = $field.get_mut($key)
            && value.is_object()
        {
            let value = std::mem::take(value);
            $field[$key] = Value::Array(vec![value]);
        }

        if let Some(Value::Array(array)) = $field.get_mut($key) {
            for entry in array {
                if let Value::Object(object) = entry {
                    stringify!(object, "id");
                    if let Some(Value::String(id)) = object.get_mut("id") {
                        if $redact.contains(id) {
                            object.remove("id");
                        } else {
                            if !id.starts_with($prefix) {
                                id.insert_str(0, &$prefix);
                            }
                            if let Some(roles) = $lookup.get_mut(id) {
                                roles.insert($role.into());
                            }
                        }
                    }
                }
            }
        }
    };
}

#[derive(Debug, Default)]
pub struct Prepare;

impl Prepare {
    ///
    /// # Errors
    ///
    /// # Panics
    ///
    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::too_many_lines)]
    // https://github.com/rust-lang/rust-clippy/issues/10413
    #[allow(clippy::significant_drop_tightening)]
    pub fn run<W: Write + Send>(
        buffer: impl BufRead + Send,
        settings: Settings,
        output: &mut W,
        errors: &mut W,
    ) -> Result<(), anyhow::Error> {
        let empty_set: HashSet<String> = HashSet::new();
        let default_mapping = HashMap::new();

        let output = Job::new(BufWriter::new(output));
        let errors = Job::new(BufWriter::new(errors));

        // [defaults]
        let defaults = settings.defaults.unwrap_or_default();
        let currency_default = defaults.currency.map(Value::String);
        let item_classification_scheme_default = defaults.item_classification_scheme.map(Value::String);
        let bid_status_default = defaults.bid_status.map(Value::String);
        let award_status_default = defaults.award_status.map(Value::String);
        let party_roles_default = defaults.party_roles.unwrap_or_default();

        // [redactions]
        let redactions = settings.redactions.unwrap_or_default();
        let mut redact_amount = redactions
            .amount
            .unwrap_or_default()
            .split_terminator('|')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect::<Vec<_>>();
        let redact_organization_id = parse_pipe_separated_value(redactions.organization_id);
        // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by
        redact_amount.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // [corrections]
        let corrections = settings.corrections.unwrap_or_default();
        let award_status_by_contract_status = corrections.award_status_by_contract_status.unwrap_or_default();

        // [modifications]
        let modifications = settings.modifications.unwrap_or_default();
        let move_auctions = modifications.move_auctions.unwrap_or_default();
        let binding = modifications.prefix_buyer_or_procuring_entity_id.unwrap_or_default();
        let prefix_buyer_or_procuring_entity_id = binding.as_str();
        let binding = modifications.prefix_tenderer_or_supplier_id.unwrap_or_default();
        let prefix_tenderer_or_supplier_id = binding.as_str();
        let split_procurement_method_details = modifications.split_procurement_method_details;

        // [codelists.*]
        let codelists = settings.codelists.unwrap_or_default();
        let bid_status = codelists.get(&Codelist::BidStatus).unwrap_or(&default_mapping);
        let award_status = codelists.get(&Codelist::AwardStatus).unwrap_or(&default_mapping);

        let result = buffer
            .lines()
            .enumerate()
            .par_bridge()
            .try_for_each(|(i, lines)| -> Result<(), anyhow::Error> {
                // Use guard clauses to reduce indentation and ease readabaility.
                let string = match lines {
                    Ok(string) => string,
                    Err(e) => return Ok(warn!("Line {} caused an I/O error, skipping. [{e}]", i + 1)),
                };

                let mut value: Value = match serde_json::from_str(&string) {
                    Ok(value) => value,
                    Err(e) => {
                        if !string.as_bytes().iter().all(u8::is_ascii_whitespace) {
                            warn!("Line {} is invalid JSON, skipping. [{e}]", i + 1);
                        }
                        return Ok(());
                    }
                };

                let Some(release) = value.as_object_mut() else {
                    return Ok(warn!("Line {} is not a JSON object, skipping.", i + 1));
                };

                let mut rows = csv::Writer::from_writer(errors.new_task());

                let mut award_id_contracts_cancelled = HashMap::new();

                // /ocid
                let ocid = release
                    .get("ocid")
                    .map_or_else(|| Value::Null, std::clone::Clone::clone);

                // /parties
                prepare_id_array!(
                    release,
                    "parties",
                    redact_organization_id,
                    "",
                    HashMap::<String, HashSet<String>>::new(),
                    ""
                );

                let mut party_roles_lookup = HashMap::new();
                if party_roles_default {
                    if let Some(Value::Array(parties)) = release.get("parties") {
                        for party in parties {
                            if let Some(Value::String(id)) = party.get("id") {
                                let mut set = HashSet::new();
                                if let Some(Value::Array(roles)) = party.get("roles") {
                                    for role in roles {
                                        if let Value::String(string) = role {
                                            set.insert(string.clone());
                                        }
                                    }
                                }
                                party_roles_lookup.insert(id.clone(), set);
                            }
                        }
                    }
                }

                // /buyer
                prepare_id_object!(
                    release,
                    "buyer",
                    redact_organization_id,
                    prefix_buyer_or_procuring_entity_id,
                    party_roles_lookup,
                    "buyer"
                );

                // /tender
                if let Some(Value::Object(tender)) = release.get_mut("tender") {
                    prepare_id_object!(
                        tender,
                        "procuringEntity",
                        redact_organization_id,
                        prefix_buyer_or_procuring_entity_id,
                        party_roles_lookup,
                        "procuringEntity"
                    );

                    if let Some(pat) = &split_procurement_method_details
                        && let Some(Value::String(procurement_method_details)) =
                            tender.get_mut("procurementMethodDetails")
                    {
                        *procurement_method_details =
                            procurement_method_details.split(pat).next().unwrap().trim_end().into();
                    }
                }

                // /auctions
                if move_auctions {
                    if release.contains_key("auctions") && release.contains_key("bids") {
                        warn!("Can't move /auctions, because /bids is occupied.");
                    } else if let Some(Value::Array(auctions)) = release.get_mut("auctions") {
                        let mut bids = Map::new();
                        let mut details = vec![];
                        for auction in auctions.iter_mut() {
                            if let Some(Value::Array(stages)) = auction.get_mut("stages") {
                                for stage in stages.iter_mut() {
                                    if let Some(object) = stage.as_object_mut()
                                        && let Some(Value::Array(auctions_bids)) = object.remove("bids")
                                    {
                                        details.extend(auctions_bids);
                                    }
                                }
                            }
                        }
                        bids.insert("details".into(), Value::Array(details));
                        release.insert("bids".into(), Value::Object(bids));
                    }
                }

                // /bids
                if let Some(Value::Object(bids)) = release.get_mut("bids")
                    && let Some(Value::Array(details)) = bids.get_mut("details")
                {
                    for (j, bid) in details.iter_mut().enumerate() {
                        if let Some(Value::Object(value)) = bid.get_mut("value") {
                            if let Some(Value::Number(amount)) = value.get("amount")
                                && let Some(amount) = amount.as_f64()
                            {
                                if redact_amount
                                    .binary_search_by(|probe| probe.partial_cmp(&amount).unwrap())
                                    .is_ok()
                                {
                                    value.remove("amount");
                                } else if amount == 0.0 {
                                    rows.serialize((i + 1, &ocid, "/bids/details[]/value/amount", j, "", "is zero"))?;
                                }
                            }
                            if !value.contains_key("currency") {
                                if let Some(default) = &currency_default {
                                    value.insert("currency".into(), default.clone());
                                } else {
                                    rows.serialize((i + 1, &ocid, "/bids/details[]/value/currency", j, "", "not set"))?;
                                }
                            }
                        }

                        if let Some(Value::Array(items)) = bid.get_mut("items") {
                            for (k, item) in items.iter_mut().enumerate() {
                                if let Some(Value::Object(classification)) = item.get_mut("classification")
                                    && !classification.contains_key("scheme")
                                {
                                    if let Some(default) = &item_classification_scheme_default {
                                        classification.insert("scheme".into(), default.clone());
                                    } else {
                                        rows.serialize((
                                            i + 1,
                                            &ocid,
                                            "/bids/details[]/items[]/classification/scheme",
                                            format!("{j}.{k}"),
                                            "",
                                            "not set",
                                        ))?;
                                    }
                                }
                            }
                        }

                        // is_none() is used instead of !contains_key(), as bid is a Value, not a Map.
                        if bid.get("status").is_none() {
                            if let Some(default) = &bid_status_default {
                                bid["status"] = default.clone();
                            } else {
                                rows.serialize((i + 1, &ocid, "/bids/details[]/status", j, "", "not set"))?;
                            }
                        }
                        if let Some(Value::String(status)) = bid.get_mut("status") {
                            if bid_status.contains_key(status) {
                                status.clone_from(&bid_status[status]);
                            }
                            if !BID_STATUS.contains(status.as_str()) {
                                rows.serialize((i + 1, &ocid, "/bids/details[]/status", j, status, "invalid"))?;
                            }
                        }

                        prepare_id_array!(
                            bid,
                            "tenderers",
                            redact_organization_id,
                            prefix_tenderer_or_supplier_id,
                            party_roles_lookup,
                            "tenderer"
                        );
                    }
                }

                // /contracts
                if award_status_by_contract_status && let Some(Value::Array(contracts)) = release.get_mut("contracts") {
                    for contract in &mut *contracts {
                        stringify!(contract, "awardID");

                        if let Some(Value::String(award_id)) = contract.get("awardID") {
                            let is_cancelled = is_status!(contract, "cancelled");
                            award_id_contracts_cancelled
                                .entry(award_id.clone())
                                .and_modify(|b| *b &= is_cancelled)
                                .or_insert(is_cancelled);
                        }
                    }
                }

                // /awards
                if let Some(Value::Array(awards)) = release.get_mut("awards") {
                    for (j, award) in awards.iter_mut().enumerate() {
                        stringify!(award, "id");

                        if let Some(Value::Array(items)) = award.get_mut("items") {
                            for (k, item) in items.iter_mut().enumerate() {
                                prepare_id_object!(
                                    item,
                                    "classification",
                                    empty_set,
                                    "",
                                    HashMap::<String, HashSet<String>>::new(),
                                    ""
                                );

                                if let Some(Value::Object(classification)) = item.get_mut("classification")
                                    && !classification.contains_key("scheme")
                                {
                                    if let Some(default) = &item_classification_scheme_default {
                                        classification.insert("scheme".into(), default.clone());
                                    } else {
                                        rows.serialize((
                                            i + 1,
                                            &ocid,
                                            "/awards[]/items[]/classification/scheme",
                                            format!("{j}.{k}"),
                                            "",
                                            "not set",
                                        ))?;
                                    }
                                }
                            }
                        }

                        // is_none() is used instead of !contains_key(), as award is a Value, not a Map.
                        if award.get("status").is_none() {
                            if let Some(default) = &award_status_default {
                                award["status"] = default.clone();
                            } else {
                                rows.serialize((i + 1, &ocid, "/awards[]/status", j, "", "not set"))?;
                            }
                        }
                        if let Some(Value::String(status)) = award.get_mut("status") {
                            if award_status.contains_key(status) {
                                status.clone_from(&award_status[status]);
                            }
                            if !AWARD_STATUS.contains(status.as_str()) {
                                rows.serialize((i + 1, &ocid, "/awards[]/status", j, status, "invalid"))?;
                            }
                        }
                        if award_status_by_contract_status
                            && let Some(Value::String(id)) = award.get("id")
                            && *award_id_contracts_cancelled.get(id).unwrap_or(&false)
                        {
                            award["status"] = Value::String("cancelled".into());
                        }

                        prepare_id_array!(
                            award,
                            "suppliers",
                            redact_organization_id,
                            prefix_tenderer_or_supplier_id,
                            party_roles_lookup,
                            "supplier"
                        );
                    }
                }

                if party_roles_default {
                    if let Some(Value::Array(parties)) = release.get_mut("parties") {
                        for party in parties.iter_mut() {
                            if let Some(Value::String(id)) = party.get("id") {
                                // Don't `std::mem::take` in case `/parties[]/id` repeats.
                                let mut roles: Vec<_> = party_roles_lookup[id].clone().into_iter().collect();
                                if !roles.is_empty() {
                                    roles.sort_unstable();
                                    party["roles"] = Value::Array(roles.into_iter().map(Value::String).collect());
                                }
                            }
                        }
                    }
                }

                writeln!(output.new_task(), "{}", &serde_json::to_string(&release)?)?;

                Ok(())
            });

        // Buffers flush when dropped, but any errors are ignored. Flush explicitly to raise errors.
        output.new_task().flush()?;
        errors.new_task().flush()?;

        result
    }
}

#[derive(Debug, Default)]
pub struct Coverage {
    counts: IndexMap<String, u32>,
}

impl Coverage {
    pub const fn results(&self) -> &IndexMap<String, u32> {
        &self.counts
    }

    ///
    /// # Errors
    ///
    pub fn run(buffer: impl BufRead + Send) -> Result<Self> {
        fold_reduce(
            buffer,
            Self::default,
            |mut item, value| {
                item.add(value, &mut Vec::with_capacity(16));
                item
            },
            |mut item, other| {
                for (k, v) in other.counts {
                    item.increment(k, v);
                }
                item
            },
            Ok,
        )
    }

    // The longest path has 6 parts (as below or contracts/implementation/transactions/payer/identifier/id).
    // The longest pointer has 10 parts (contracts/0/amendments/0/unstructuredChanges/0/oldValue/classifications/0/id).
    fn add(&mut self, value: Value, path: &mut Vec<String>) -> bool {
        let mut increment = false;

        // Using a String as the key with `join("/")` is faster than Vec<String> as the key with `to_vec()`.
        match value {
            Value::Null => {}
            Value::Array(vec) => {
                if !vec.is_empty() {
                    path.push("[]".into());
                    for item in vec {
                        increment |= self.add(item, path);
                    }
                    path.pop();
                }
            }
            Value::Object(map) => {
                if !map.is_empty() {
                    path.push("/".into());
                    for (k, v) in map {
                        path.push(k);
                        increment |= self.add(v, path);
                        path.pop();
                    }
                    if increment {
                        self.increment(path.join(""), 1);
                    }
                    path.pop();
                }
            }
            Value::String(string) => {
                increment = !string.is_empty();
            }
            // number, boolean
            _ => {
                increment = true;
            }
        }

        if increment {
            self.increment(path.join(""), 1);
        }
        increment
    }

    fn increment(&mut self, path: String, delta: u32) {
        self.counts
            .entry(path)
            .and_modify(|count| *count += delta)
            .or_insert(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::Path;

    use config::Config;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    #[cfg(test)]
    #[ctor::ctor]
    fn setup() {
        // Consistent `prepare` output.
        env::set_var("RAYON_NUM_THREADS", "1");
    }

    fn reader(stem: &str, extension: &str) -> BufReader<File> {
        let path = format!("tests/fixtures/{stem}.{extension}");
        let file = File::open(path).unwrap();

        BufReader::new(file)
    }

    fn check_coverage(name: &str) {
        let result = Coverage::run(reader(name, "jsonl"));
        let expected: IndexMap<String, u32> = serde_json::from_reader(reader(name, "expected")).unwrap();

        assert_eq!(result.unwrap().counts, expected);
    }

    fn check_prepare(name: &str) {
        let mut output = vec![];
        let mut errors = vec![];
        let basedir = Path::new("tests").join("fixtures");
        let source = config::File::from(basedir.join(format!("{name}.ini")));

        // Same as main.rs.
        let config = Config::builder().add_source(source).build().unwrap();
        let settings = serde_path_to_error::deserialize(config).unwrap();

        let result = Prepare::run(reader(name, "jsonl"), settings, &mut output, &mut errors);

        let mut expected_output = String::new();
        reader(name, "output").read_to_string(&mut expected_output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), expected_output);

        if basedir.join(format!("{name}.errors")).exists() {
            let mut expected_errors = String::new();
            reader(name, "errors").read_to_string(&mut expected_errors).unwrap();
            assert_eq!(String::from_utf8(errors).unwrap(), expected_errors);
        }

        assert!(result.is_ok());
    }

    fn check_indicators(name: &str, settings: Settings) {
        let result = Indicators::run(reader(name, "jsonl"), settings, &false);
        let expected: IndexMap<Group, IndexMap<String, HashMap<Indicator, f64>>> =
            serde_json::from_reader(reader(name, "expected")).unwrap();

        assert_eq!(result.unwrap().results, expected);
    }

    #[rstest]
    #[case("X", false, false, true)]
    #[case("X", true, false, false)]
    #[case("X", false, true, true)]
    #[case("Y", false, false, true)]
    #[case("Y", true, false, true)]
    #[case("Y", false, true, true)]
    #[case("N", false, false, true)]
    #[case("N", true, false, false)]
    #[case("N", false, true, false)]
    fn matches_procurement_method_details(
        #[case] value: &str,
        #[case] include: bool,
        #[case] exclude: bool,
        #[case] flagged: bool,
    ) {
        let data = json!({
            "ocid": "F",
            "bids": {
                "details": [
                    {
                        "status": "disqualified",
                        "value": {
                            "amount": 1,
                            "currency": "USD"
                        }
                    },
                    {
                        "status": "valid",
                        "value": {
                            "amount": 2,
                            "currency": "USD"
                        }
                    }
                ]
            },
            "awards": [
                {
                    "status": "active"
                }
            ],
            "tender": {
                "procurementMethodDetails": value
            }
        });

        let settings = Settings {
            price_comparison_procurement_methods: if include { Some(String::from("Y")) } else { None },
            no_price_comparison_procurement_methods: if exclude { Some(String::from("N")) } else { None },
            R036: Some(Default::default()),
            ..Default::default()
        };

        let mut bytes: Vec<u8> = vec![];
        serde_json::to_writer(&mut bytes, &data).unwrap();

        let result = Indicators::run(BufReader::new(&*bytes), settings, &false);

        assert_eq!(
            result.unwrap().results,
            if flagged {
                IndexMap::from([(
                    Group::OCID,
                    IndexMap::from([(String::from("F"), HashMap::from([(Indicator::R036, 1.0)]))]),
                )])
            } else {
                IndexMap::new()
            }
        );
    }

    include!(concat!(env!("OUT_DIR"), "/lib.include"));
}
