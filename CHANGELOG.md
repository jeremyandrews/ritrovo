# Changelog

## Unreleased

- `ritrovo_cfp` 1.1.0: the CFP badge counted down from the item's last save (`changed`), not today; it now reads the database clock. Also parses the importer's `YYYY-MM-DD` deadlines, which previously showed no badge.
- `ritrovo_importer` 1.2.0: updating a conference wrote an empty `field_source_id`, so the next import could not find it and inserted a duplicate; the key is now written back, and migration 004 restores blanked keys and merges the duplicates.
