# Changelog

## Unreleased

- **`ritrovo_cfp` 1.1.0: the CFP badge counts down from today, not from the
  day the conference was last saved.**

  Root cause: `tap_item_view` measured the time remaining as
  `field_cfp_end_date - item.changed`, on the stated grounds that plugins have
  no clock. `changed` is the record's last-modified time, so the badge described
  that day rather than the day it was viewed. A conference untouched for months
  kept counting down from its last save, and a CFP that had already closed still
  read "CFP Open". Nothing looked wrong on a freshly imported record, because on
  the day of import `changed` and now are the same instant, which is how the
  fault survived its own tests: every test built its item with `changed` equal
  to the "now" it assumed.

  The premise was half right. The kernel links no WASI clock into a plugin, so
  `std::time` is not available; but the database clock is, and
  `ritrovo_importer` already reads it. The badge now reads the same clock at
  render time, only when the conference has a deadline, and renders no badge if
  the clock cannot be read rather than guessing. The manifest declares
  `host_interfaces = ["db", "logging"]` and `raw_sql = true` for that one query.

  The countdown itself is now a pure function of the item and an instant, and
  the regression tests pin both faces of the bug at that layer: a conference
  whose `changed` is long before a passed deadline renders no badge, and one
  whose deadline is three days out renders the urgent state whatever `changed`
  says.

  Two faults in the same function, found while testing it against imported
  data, are fixed with it:

  - The badge parsed `field_cfp_end_date` only as an integer timestamp, but
    `ritrovo_importer` stores it as a `YYYY-MM-DD` date. Every imported
    conference failed the parse, so no badge ever rendered on the demo site. A
    date now resolves to the end of that day, UTC, since a CFP that closes on a
    date is open through it.
  - A deadline between 24 and 48 hours away read "CFP Closes Today!". It now
    reads "1 day left", and "Closes Today!" means today.
