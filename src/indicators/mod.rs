pub mod r003;
pub mod r018;
pub mod r024;
pub mod r025;
pub mod r028;
pub mod r030;
pub mod r035;
pub mod r036;
pub mod r038;
pub mod r048;
pub mod r055;
pub mod r058;
pub mod util;

use std::collections::{HashMap, HashSet};
use std::ops::AddAssign;

use chrono::NaiveDate;
use indexmap::IndexMap;
use serde::ser::SerializeMap;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

// Settings.

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all(deserialize = "snake_case"))]
pub enum Codelist {
    BidStatus,
    AwardStatus,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    pub currency: Option<String>,
    pub item_classification_scheme: Option<String>,
    pub bid_status: Option<String>,
    pub award_status: Option<String>,
    pub party_roles: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Redactions {
    pub amount: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corrections {
    pub award_status_by_contract_status: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modifications {
    pub move_auctions: Option<bool>,
    pub prefix_buyer_or_procuring_entity_id: Option<String>,
    pub prefix_tenderer_or_supplier_id: Option<String>,
    pub split_procurement_method_details: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusions {
    pub procurement_method_details: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatThreshold {
    threshold: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerThreshold {
    threshold: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R003 {
    pub threshold: Option<i64>,
    pub procurement_methods: Option<String>,
    pub procurement_method_details: Option<HashMap<String, i64>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R018 {
    pub procurement_methods: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R025 {
    percentile: Option<usize>,
    threshold: Option<f64>, // ratio
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R038 {
    threshold: Option<f64>, // ratio
    minimum_submitted_bids: Option<usize>,
    minimum_contracting_processes: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R048 {
    pub digits: Option<usize>,
    pub threshold: Option<usize>,
    pub minimum_contracting_processes: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R055 {
    pub threshold: Option<f64>,
    #[serde(deserialize_with = "naive_date_from_str")]
    pub start_date: Option<NaiveDate>,
    #[serde(deserialize_with = "naive_date_from_str")]
    pub end_date: Option<NaiveDate>,
}

// https://serde.rs/field-attrs.html#deserialize_with
fn naive_date_from_str<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    s.map_or_else(
        || Ok(None),
        |s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(Some)
                .map_err(de::Error::custom)
        },
    )
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
pub struct Settings {
    // prepare command.
    pub codelists: Option<HashMap<Codelist, HashMap<String, String>>>,
    pub defaults: Option<Defaults>,
    pub redactions: Option<Redactions>,
    pub corrections: Option<Corrections>,
    pub modifications: Option<Modifications>,
    // indicators command.
    pub currency: Option<String>,
    pub no_price_comparison_procurement_methods: Option<String>,
    pub price_comparison_procurement_methods: Option<String>,
    pub exclusions: Option<Exclusions>,
    pub R003: Option<R003>,
    pub R018: Option<R018>,
    pub R024: Option<FloatThreshold>, // ratio
    pub R025: Option<R025>,
    pub R028: Option<Empty>,
    pub R030: Option<Empty>,
    pub R035: Option<IntegerThreshold>, // count
    pub R036: Option<Empty>,
    pub R038: Option<R038>,
    pub R048: Option<R048>,
    pub R055: Option<R055>,
    pub R058: Option<FloatThreshold>, // ratio
}

// Final results.

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Group {
    OCID,
    Buyer,
    ProcuringEntity,
    Tenderer,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Indicator {
    R003,
    R018,
    R024,
    R025,
    R028,
    R030,
    R035,
    R036,
    R038,
    R048,
    R055,
    R058,
}

#[derive(Debug, Default, Serialize)]
pub struct Maps {
    /// The buyer for each `ocid` in which at least one bid is disqualified.
    pub ocid_buyer_r038: HashMap<String, String>,
    /// The procuring entity for each `ocid` in which at least one bid is disqualified.
    pub ocid_procuringentity_r038: HashMap<String, String>,
    /// The tenderers that submitted bids for each `ocid`.
    pub ocid_tenderer: HashMap<String, HashSet<String>>,
    /// The flagged tenderers for each flagged `ocid`.
    pub ocid_tenderer_r024: HashMap<String, HashSet<String>>,
    pub ocid_tenderer_r028: HashMap<String, HashSet<String>>,
    pub ocid_tenderer_r030: HashMap<String, HashSet<String>>,
    pub ocid_tenderer_r035: HashMap<String, HashSet<String>>,
    pub ocid_tenderer_r058: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Default)]
pub struct Fraction {
    numerator: usize,
    denominator: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct RoundMap(#[serde(serialize_with = "round")] IndexMap<String, f64>);

#[derive(Debug, Default)]
pub struct Indicators {
    pub results: IndexMap<Group, IndexMap<String, HashMap<Indicator, f64>>>,
    pub meta: HashMap<Indicator, RoundMap>,
    pub maps: Maps,
    pub currency: Option<String>,
    /// The percentage difference between the winning bid and the second-lowest valid bid for each `ocid`.
    pub second_lowest_bid_ratios: HashMap<String, f64>,
    pub winner_and_lowest_non_winner: HashMap<String, [String; 2]>,
    /// The ratio of winning bids to submitted bids for each `bids/details/tenderers/id`.
    pub r025_tenderer: HashMap<String, Fraction>,
    /// The ratio of disqualified bids to submitted bids for each `buyer/id`.
    pub r038_buyer: HashMap<String, Fraction>,
    /// The ratio of disqualified bids to submitted bids for each `tender/procuringEntity/id`.
    pub r038_procuring_entity: HashMap<String, Fraction>,
    /// The ratio of disqualified bids to submitted bids for each `bids/details/tenderers/id`.
    pub r038_tenderer: HashMap<String, Fraction>,
    /// The item classifications for each `bids/details/tenderers/id`.
    pub r048_classifications: HashMap<String, (usize, HashSet<String>)>,
    /// The `tender/value/amount` for each `ocid` when `tender/procurementMethod` is 'open'.
    pub r055_open_tender_amount: HashMap<String, f64>,
    /// The total awarded amount for each `buyer/id` and `awards/suppliers/id` when `tender/procurementMethod` is 'direct'.
    pub r055_direct_awarded_amount_supplier_buyer: HashMap<(String, String), f64>,
    /// The total awarded amount for each `tender/procuringEntity/id` and `awards/suppliers/id` when `tender/procurementMethod` is 'direct'.    
    pub r055_direct_awarded_amount_supplier_procuring_entity: HashMap<(String, String), f64>,
    /// Whether to map contracting processes to organizations.
    pub map: bool,
}

fn round<S>(m: &IndexMap<String, f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(m.len()))?;
    for (k, v) in m {
        map.serialize_entry(k, &((v * 10_000.0).round() / 10_000.0))?;
    }
    map.end()
}

// Traits.

pub trait Calculate {
    fn new(settings: &mut Settings) -> Self
    where
        Self: Sized;

    #[allow(unused_variables)]
    fn fold(&self, item: &mut Indicators, release: &Map<String, Value>, ocid: &str) {}

    #[allow(unused_variables)]
    fn reduce(&self, item: &mut Indicators, other: &mut Indicators) {}

    #[allow(unused_variables)]
    fn finalize(&self, item: &mut Indicators) {}
}

// Trait implementations.

// https://en.wikipedia.org/wiki/Mediant_(mathematics)
impl AddAssign for Fraction {
    fn add_assign(&mut self, other: Self) {
        self.numerator += other.numerator;
        self.denominator += other.denominator;
    }
}

impl From<Fraction> for f64 {
    fn from(fraction: Fraction) -> Self {
        fraction.numerator as Self / fraction.denominator as Self
    }
}

impl From<&Fraction> for f64 {
    fn from(fraction: &Fraction) -> Self {
        fraction.numerator as Self / fraction.denominator as Self
    }
}

// Initialization macros.

macro_rules! fraction {
    ( $numerator:expr , $denominator:expr ) => {
        crate::indicators::Fraction {
            numerator: $numerator,
            denominator: $denominator,
        }
    };
}
pub(crate) use fraction;

// Macros to access struct fields dynamically (.$).

macro_rules! sum {
    ( $accumulator:ident , $current:ident , $field:ident ) => {
        for (key, value) in std::mem::take(&mut $current.$field) {
            let fraction = $accumulator.$field.entry(key).or_default();
            *fraction += value;
        }
    };
}
pub(crate) use sum;

macro_rules! set_tenderer_map {
    ( $item:ident , $field:ident , $ocid:expr , $id:expr ) => {
        if $item.map {
            $item.maps.$field.entry($ocid).or_default().insert($id);
        }
    };
}
pub(crate) use set_tenderer_map;

macro_rules! reduce_map {
    ( $item:ident , $other:ident , $field:ident ) => {
        if $item.map {
            // If each OCID appears on only one line of the file, no overwriting will occur.
            $item.maps.$field.extend(std::mem::take(&mut $other.maps.$field));
        }
    };
}
pub(crate) use reduce_map;

// Other macros.

// IndexMap<Group, IndexMap<String, HashMap<Indicator, f64>>>
macro_rules! set_result {
    ( $item:ident , $group:ident , $key:expr , $indicator:ident , $value:expr ) => {
        $item
            .results
            .entry(crate::indicators::Group::$group)
            .or_default()
            .entry($key.to_owned())
            .or_default()
            .insert(crate::indicators::Indicator::$indicator, $value)
    };
}
pub(crate) use set_result;

macro_rules! set_meta {
    ( $item:ident , $indicator:ident , $key:expr , $value:expr ) => {
        $item
            .meta
            .entry(crate::indicators::Indicator::$indicator)
            .or_default()
            .0
            .insert($key.to_owned(), $value)
    };
}
pub(crate) use set_meta;

macro_rules! is_status {
    ( $object:ident , $status:expr ) => {
        $object
            .get("status")
            .map_or(false, |status| status.as_str() == Some($status))
    };
}
pub(crate) use is_status;
