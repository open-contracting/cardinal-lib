# indicators

The `indicators` command calculates procurement indicators and red flags.

Run the `help` command to read its description, output format and options:

```console
$ ocdscardinal help indicators
Calculate procurement indicators from OCDS compiled releases in a line-delimited JSON file

The result is a JSON object, in which the keys are one of `OCID`, `Buyer`, `ProcuringEntity` or
`Tenderer`. The values are JSON objects, in which the keys are identifiers (e.g. ocid) and values
are results (of any indicators that returned a result).

Unless --no-meta is set, the result has a "Meta" key, with information about the quartiles and
fences used to calculate the results.

If --map is set, the result has a "Maps" key, with mappings from contracting processes to
organizations.

Usage: ocdscardinal[EXE] indicators [OPTIONS] <FILE>

Arguments:
  <FILE>
          The path to the file (or "-" for standard input), in which each line is a contracting
          process as JSON text

Options:
  -s, --settings <SETTINGS>
          The path to the settings file

  -v, --verbose...
          Increase verbosity

  -c, --count
          Print the number of results per group to standard error

      --no-meta
          Exclude the "Meta" key from the results object

      --map
          Include the "Maps" key, mapping contracting processes to organizations

  -h, --help
          Print help (see a summary with '-h')

```

(indicators-workflow)=
## Workflow

:::{attention}
Before following this command's workflow, follow the earlier steps in the {doc}`../../topics/workflow`.
:::

1. **Select indicators**. If you ran the {doc}`../init` command when preparing your data, you already have a {doc}`../../topics/settings` that enables all indicators. [Enable](#enable-an-indicator) or [disable](#disable-an-indicator) indicators as you wish.
1. **Run the command**. For example, if your settings are in `settings.ini` and your data is in `prepared.jsonl`, this command writes the output to `results.json`:

   ```bash
   ocdscardinal indicators --settings settings.ini prepared.jsonl > results.json
   ```

1. **Review the results**. Read the [demonstration](#demonstration) to learn about the output format.

   :::{admonition} To do
   :class: caution
   This section will expand on the interpretation of results. See GitHub issue [#40](https://github.com/open-contracting/cardinal-rs/issues/40).
   :::

1. **Edit the settings**. Adjust the configuration of the [indicators](#list) based on your review of the results, in order to reduce false positives.

Repeat the last three steps until you are satisfied with the results.

:::{note}
Have questions, concerns, or feedback? [Email James McKinney](mailto:jmckinney@open-contracting.org), OCP's Head of Technology.
:::

(indicators-demo)=
## Demonstration

A procurement indicator or red flag can be about a contracting process, buyer, procuring entity or tenderer. For example, a contracting process might have a suspicious feature, like all bids except the winner's being disqualified. Or, a buyer might exhibit suspicious behavior, like disqualifying a large number of bids across its contracting processes.

The JSON output is therefore organized as an object in which the key is a **group**: "OCID" (the unique identifier of a contracting process), "Buyer", "ProcuringEntity" or "Tenderer". For example:

```json
{
  "OCID": {},
  "Buyer": {},
  "ProcuringEntity": {},
  "Tenderer": {}
}
```

Each value at the *top* level is an object representing the results within that **group**, in which the key is an **identifier** extracted from the input data:

| Group | Identifier |
| - | - |
| OCID | `/ocid` |
| Buyer | `/buyer/id` |
| ProcuringEntity | `/tender/procuringEntity/id` |
| Tenderer | `/bids/details[]/tenderers[]/id` |

For example:

```json
{
  "OCID": {
    "ocds-6550wx-JRFPFA-DAF-CM-2021-0012": {}
  },
  "Buyer": {
    "DO-RPE-55216": {}
  }
}
```

Each value at the *second* level is an object representing the results relating to that **identifier**, in which the key is the **code** for an indicator, and the value is the **output** of that indicator. For example:

```json
{
  "OCID": {
    "ocds-6550wx-JRFPFA-DAF-CM-2021-0012": {
      "R036": 1.0
    }
  },
  "Buyer": {
    "DO-RPE-55216": {
      "R038": 0.8
    }
  }
}
```

The **output** of an indicator is always a decimal. If an indicator didn't produce an output – either because it couldn't be calculated, or because no red flag was raised – then its code won't appear.

You can [consult](#list) the codes for all indicators, read the description of their outputs and see a demonstration of their calculation.

The JSON output also has a ``Meta`` key at the top level. Its value is an object with information about the quartiles and fences used to calculate the results, rounded to 4 decimals. For example:

```json
{
  // ...
  "Meta": {
    "R024": {
      "q1": 66.6667,
      "q3": 100.0,
      "lower_fence": 16.6667
    }
  }
}
```

(indicators-config)=
## Configuration

The page for each [indicator](#list) describes its individual settings.

All configuration is optional. Cardinal provides good defaults.

:::{seealso}
An introduction to the {doc}`../../topics/settings`.
:::

(enable-an-indicator)=
### Enable an indicator

To enable an indicator, start a section with its code, for example:

```ini
[R024]
```

You don't need to set properties in this section. (Only if you want to!)

### Disable an indicator

The disable an indicator, either delete its section and properties, or comment them out, for example:

```ini
; [R024]
; threshold = 0.05
```

Now, the `indicators` command won't run this indicator.

(global-configuration)=
### Global configuration

You might want to exclude some procedures from all indicators, whether for methodological reasons or due to quality issues. To exclude procedures based on the value of `/tender/procurementMethodDetails`, add to your settings file, as a pipe-separated list, for example:

```ini
[exclusions]
procurement_method_details = Random Selection|Sorteo de Obras
```

Some indicators compares bid prices. However, for some procedures, it is inappropriate to compare bid prices; for example, if the buyer predetermines the price, then the indicator for prices close to the winning bid will return a false positive.

Procedures can be included or excluded based on the value of `/tender/procurementMethodDetails`.

To exclude procedures from price comparisons, add to the top of your settings file, as a pipe-separated list, for example:

```ini
no_price_comparison_procurement_methods = Random Selection|Sorteo de Obras
```

Alternatively, to include procedures in price comparisons, add to the top of your settings file, as a pipe-separated list, for example:

```ini
price_comparison_procurement_methods = Reverse Auction
```

A procedure is excluded if either:

-  `no_price_comparison_procurement_methods` is set, and `/tender/procurementMethodDetails` is set and matches
-  `price_comparison_procurement_methods` is set, and `/tender/procurementMethodDetails` isn't set or doesn't match

## Glossary

% Do not add terms to the glossary that are not used in the documentation!

:::{glossary}
bid

  An offer made by an {term}`economic operator` as part of a {term}`contracting process`. Also known as a *tender*.

bidder

  Synonym of {term}`tenderer`.

buyer

  The organization aiming to conclude a contract with an {term}`economic operator` or to use the goods, services or works resulting from the contract, as part of a {term}`contracting process`.

contracting process

  All the actions aimed at implementing one or more contracts. This covers tendering, awarding, contracting and implementation. Also known as a *procedure*.

economic operator

  A person or organization – or group of people or organizations – that offers goods, services or works. Also known as a *business*, *contractor*, *service provider*, *supplier* or *undertaking*.

procuring entity

  The organization managing the {term}`contracting process`. An organization can be both a {term}`buyer` and a procuring entity (like in a simple contracting process).

submitted

  A {term}`bid` is submitted if its status is pending (i.e. not evaluated yet), valid (i.e. qualified), or disqualified. It is not submitted if its status is invited or withdrawn.

tenderer

  An {term}`economic operator` that {term}`submitted` one or more {term}`bids<bid>` as part of a {term}`contracting process`.

winner

  A {term}`tenderer` that is awarded a contract.
:::

(list)=
## Indicators

Indicators are assigned codes for easy reference: for example, `R001`. The first letter indicates the category: **R**ed flag or **U**se case.

The page for each indicator describes its individual methodology. For all indicators, a contracting process is excluded if:

- The `ocid` isn't a string.
- The contracting process (`/tender/status`) is cancelled.
- The relevant organization references don't set an `id`.
- Monetary values, where relevant, don't use the main currency.

  To configure the main currency, add to the top of your settings file:

  ```ini
  currency = USD
  ```

  Otherwise, the main currency is set to the first observed currency.

  :::{note}
  Do you want to eliminate this exclusion? Please contributed to [GitHub issue #11](https://github.com/open-contracting/cardinal-rs/issues/11).
  :::

:::{toctree}
:hidden: true

R/index
:::

### Red flags

:::{list-table}
:header-rows: 1

* - Code
  - Title
  - Description
* - [R003](R/003)
  - [Short submission period](R/003)
  - The submission period is too short.
* - [R018](R/018)
  - [Single bid received](R/018)
  - Only one tenderer submitted a bid.
* - [R024](R/024)
  - [Price close to winning bid](R/024)
  - The percentage difference between the winning bid and the second-lowest valid bid is a low outlier.
* - [R025](R/025)
  - [Excessive unsuccessful bids](R/025)
  - The ratio of winning bids to submitted bids for a top tenderer is a low outlier.
* - [R028](R/028)
  - [Identical bid prices](R/028)
  - Different tenderers submitted bids with the same price.
* - [R030](R/030)
  - [Late bid won](R/030)
  - The winning bid was received after the submission deadline.
* - [R035](R/035)
  - [All except winning bid disqualified](R/035)
  - Bids are disqualified if not submitted by the single tenderer of the winning bid.
* - [R036](R/036)
  - [Lowest bid disqualified](R/036)
  - The lowest submitted bid is disqualified, while the award criterion is price only.
* - [R038](R/038)
  - [Excessive disqualified bids](R/038)
  - The ratio of disqualified bids to submitted bids is a high outlier per buyer, procuring entity or tenderer.
* - [R044](R/044)
  - [Business similarities between suppliers](R/044)
  - Different tenderers bidding for the same contracting process have similar information.
* - [R048](R/048)
  - [Heterogeneous supplier](R/048)
  - The variety of items supplied by a tenderer is a high outlier.
* - [R058](R/058)
  - [Heavily discounted bid](R/058)
  - The percentage difference between the winning bid and the second-lowest valid bid is a high outlier.
:::
