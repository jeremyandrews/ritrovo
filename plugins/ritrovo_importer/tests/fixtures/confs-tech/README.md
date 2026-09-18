# confs.tech fixtures

What the importer's host-in-the-loop suite serves in place of
`https://raw.githubusercontent.com/tech-conferences/conference-data/main/conferences`.
Paths mirror the upstream layout, `<year>/<topic>.json`; anything not here
answers 404, which is what upstream answers for a topic with no file that year.

| file | what it is | why it is here |
|---|---|---|
| `2026/rust.json` | upstream, verbatim, fetched 2026-09-17 | a small real file: 5 conferences, one batch |
| `2026/data.json` | upstream, verbatim, fetched 2026-09-17 | a real file over the importer's 50-per-batch limit: 98 conferences, two batches |
| `2026/css.json` | written for the suite | TokioConf again, as upstream lists it under `rust`, so dedup has to merge a second topic into one Item; and one entry whose end date precedes its start date, which validation has to reject. It was `testing.json` until the topic tree became the brief's, which has no Testing term: a feed mapping to no term is the right thing to test elsewhere, but it cannot be the fixture that proves two topics merge |

The upstream files are from
[tech-conferences/conference-data](https://github.com/tech-conferences/conference-data),
MIT licensed. They are a snapshot and are not meant to track upstream: the
assertions count what is in these files.
