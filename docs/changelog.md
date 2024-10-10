# Changelog

## 0.0.6 (2024-08-23)

### Added

- {doc}`cli/prepare` command:
  - Add `party_roles` configuration to `[defaults]` section.
  - Add `[redactions]` section. Warn about zero-value bids.
  - Add `[corrections]` section.
  - Add `[modifications]` section.
  - Normalize `/bids/details[]/tenderers` and `/awards/suppliers` to arrays.
- {doc}`cli/indicators/index` command:
  - [R003](cli/indicators/R/003): (*Short submission period*).
  - [R018](cli/indicators/R/018): (*Single bid received*).
  - [R028](cli/indicators/R/028): (*Identical bid prices*).
  - [R030](cli/indicators/R/030): (*Late bid won*).
  - [R044](cli/indicators/R/048): (*Business similarities between suppliers*).
  - [R048](cli/indicators/R/048): (*Heterogeneous supplier*).
  - [R058](cli/indicators/R/058): (*Heavily discounted bid*).
  - Add `no_price_comparison_procurement_methods` configuration.
  - Add `price_comparison_procurement_methods` configuration.
  - Add `[exclusions]` section.

### Changed

- {doc}`cli/prepare` command:
  - Warn about invalid default statuses.
  - Use snake case (`[codelists.bid_status]`) in the settings file, instead of a mix of snake case and camel case (`[codelists.BidStatus]`).
  - Normalize `/awards[]/id` and `/contracts[]/awardID` to strings.
- {doc}`cli/indicators/index` command:
  - [R024](cli/indicators/R/024): Flag the winner and second-lowest bidder. Add `--map` key.
  - [R035](cli/indicators/R/035): Flag the winner. Add `--map` key.
  - Add global exclusion for cancelled `/tender/status`.

### Fixed

- {doc}`cli/indicators/index` command:
  - If the first quartile, third quartile and interquartile range are 0, skip the indicator to not flag 75% of cases.
  - [R024](cli/indicators/R/024): Use the lowest bid submitted by the winner as the winning bid.
  - [R036](cli/indicators/R/036): Exclude contracting processes in which no valid bid has an amount.

## 0.0.5 (2023-06-14)

### Added

- {doc}`cli/init` command:
  - Add `--force` (`-f`) option to overwrite an existing file.
- {doc}`cli/indicators/index` command:
  - Add `--map` option to include the `Maps` key.
  - Add `--no-meta` option to omit the `Meta` key.
  - [R038](cli/indicators/R/038): Add `minimum_submitted_bids` and `minimum_contracting_processes` configurations.

### Changed

- Prefix an error about an unknown property in the settings file with the path at which it occurred.
- {doc}`cli/init` command:
  - Add `currency` property to default file.
- {doc}`cli/prepare` command:
  - Improve write performance.
- {doc}`cli/indicators/index` command:
  - Add `Meta` key to assist interpretation of results.
  - The `--count` option writes to standard error instead of standard output, to not mix outputs.
  - All `threshold` and `percentile` configurations are consistently interpreted as inclusive.

## 0.0.4 (2023-05-30)

### Changed

- {doc}`cli/prepare` command:
  - Add `--output` (`-o`) and `--errors` (`-e`) options, instead of using shell redirection.
  - Fill in `/awards[]/items[]/classification/scheme` with `item_classification_scheme`.

## 0.0.3 (2023-05-29)

### Added

- {doc}`cli/init` command.
- {doc}`cli/prepare` command.
  - `[defaults]` section.
  - `[codelists.*]` sections.
- {doc}`cli/indicators/index` command:
  - [R025](cli/indicators/R/025) (*Excessive unsuccessful bids*).
  - [R036](cli/indicators/R/036) (*Lowest bid disqualified*).
  - [R038](cli/indicators/R/038) (*Excessive disqualified bids*).
- Expand documentation.

### Changed

- Error on unknown configurations in the settings file.
- {doc}`cli/indicators/index` command:
  - {ref}`Enable<enable-an-indicator>` indicators in the settings file.
  - Rename indicators from `NF###` to `R###`.
  - Remove "OCID" from output if no OCIDs reported.
  - Split indicators into trait objects.
  - Preserve top-level key order in the JSON output.
- {doc}`cli/coverage` command:
  - Preserve JSON path key order in the JSON output.

### Fixed

- Commands no longer error on `SIGPIPE` signal.

## 0.0.2 (2023-02-13)

### Added

- {doc}`cli/indicators/index` command:
  - [R035](cli/indicators/R/035) (*All except winning bid disqualified*).
  - Add `--settings SETTINGS` option for the settings file.
  - Add documentation.

## 0.0.1 (2023-02-13)

First release, including:

- [R024](cli/indicators/R/024) (*Price close to winning bid*).
