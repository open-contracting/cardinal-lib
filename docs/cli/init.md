# init

The `init` command writes a default settings file for configuration.

Run the `help` command to read its description:

```console
$ ocdscardinal help init
Write a default settings file for configuration

Usage: ocdscardinal[EXE] init [OPTIONS] <FILE>

Arguments:
  <FILE>  The path to the settings file to write

Options:
  -f, --force       Overwrite the settings file if it already exists
  -v, --verbose...  Increase verbosity
  -h, --help        Print help

```

:::{seealso}
An introduction to the {doc}`../topics/settings`.
:::

## Demonstration

Write a settings file to `settings.ini`:

```console
$ ocdscardinal init settings.ini
Settings written to "settings.ini".

```

Preview the content of the settings file:

```console
$ ocdscardinal init -
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

[R048]
; digits = 2
; threshold = 10
; minimum_contracting_processes = 20

[R055]
; threshold = 66593
; start_date = 2022-01-01
; end_date = 2022-12-31

[R058]
; threshold = 0.5

```
