//! The two values search needs that the kernel's indexers cannot read on their own.
//!
//! Both of Trovato's search indexes read text out of `item.fields` by the field
//! names `search_field_config` lists, and two of the brief's four weighted
//! fields are not text in `item.fields`:
//!
//! - **The title.** It is a column, not a field. The tsvector trigger weights the
//!   column itself, so server search finds titles; the Pagefind export does not.
//!   It writes the title into an `<h1>` OUTSIDE the `data-pagefind-body` element,
//!   and Pagefind indexes only inside that element once one exists, so on the
//!   demo a visitor searching "Codemotion" or a speaker's name got "No results
//!   found" (FRICTION.md, G-PAGEFIND-INDEX-OMITS-TITLES). The export does read a
//!   configured field called `title` out of `fields` (`search_field_config` has
//!   had one for both types since the tutorial set), so the title is mirrored
//!   there.
//! - **The topics.** `field_topics` is an array of tag uuids. Indexed as it
//!   stands, both indexes see uuid fragments and no word anyone would type
//!   (G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS). The brief already calls this field
//!   "topics (denormalized)", weight C: `field_topic_names` is the labels of every
//!   tag on the conference and of each tag's ancestors, so a Kotlin conference is
//!   found by "JVM" and "Languages" too.
//!
//! Done in SQL, in place, on every cron call, for every item whose value is
//! missing or stale, which after the first import is usually none, so a call
//! costs two indexed scans. Raw SQL rather than `save-item` because every
//! conference already arrives by raw SQL (the importer's own insert, the
//! submission form, config import), and because nothing else would keep the two
//! values right after an editor's form save drops them
//! (G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER): the next cron call
//! puts them back.
//!
//! `changed` moves with the value. That is what tells `trovato_search` that the
//! index is stale (it compares the newest `changed` on Live against its last
//! build), so a corrected title reaches Pagefind on the same cron cycle.

use trovato_sdk::host;

/// Mirror `item.title` into `fields.title` for the two searchable types.
const MIRROR_TITLE: &str = "\
UPDATE item \
   SET fields = fields || jsonb_build_object('title', title), \
       changed = EXTRACT(EPOCH FROM NOW())::bigint \
 WHERE type IN ('conference', 'speaker') \
   AND fields->>'title' IS DISTINCT FROM title";

/// Write `fields.field_topic_names`: every tag label on a conference and on its
/// ancestors, sorted and de-duplicated so the value is stable from call to call
/// and the `IS DISTINCT FROM` guard finds nothing to do the second time.
///
/// A conference with no topics gets no key at all rather than an empty string,
/// and one whose topics were all removed loses the key.
const DENORMALIZE_TOPICS: &str = "\
WITH RECURSIVE tagged AS ( \
    SELECT i.id AS item_id, t.value::uuid AS tag_id \
      FROM item i, jsonb_array_elements_text( \
           CASE WHEN jsonb_typeof(i.fields->'field_topics') = 'array' \
                THEN i.fields->'field_topics' ELSE '[]'::jsonb END) AS t(value) \
     WHERE i.type = 'conference' \
       AND t.value ~ '^[0-9a-fA-F-]{36}$' \
), lineage AS ( \
    SELECT item_id, tag_id FROM tagged \
    UNION \
    SELECT l.item_id, h.parent_id \
      FROM lineage l JOIN category_tag_hierarchy h ON h.tag_id = l.tag_id \
     WHERE h.parent_id IS NOT NULL \
), names AS ( \
    SELECT l.item_id, string_agg(DISTINCT c.label, ' ' ORDER BY c.label) AS topic_names \
      FROM lineage l JOIN category_tag c ON c.id = l.tag_id \
     GROUP BY l.item_id \
), wanted AS ( \
    SELECT i.id, n.topic_names \
      FROM item i LEFT JOIN names n ON n.item_id = i.id \
     WHERE i.type = 'conference' \
) \
UPDATE item i \
   SET fields = CASE WHEN w.topic_names IS NULL THEN i.fields - 'field_topic_names' \
                     ELSE i.fields || jsonb_build_object('field_topic_names', w.topic_names) END, \
       changed = EXTRACT(EPOCH FROM NOW())::bigint \
  FROM wanted w \
 WHERE i.id = w.id \
   AND i.fields->>'field_topic_names' IS DISTINCT FROM w.topic_names";

/// Bring both values up to date. Returns how many items each statement
/// rewrote, or `-1` for one that failed; a failure is reported in the cron
/// output and retried on the next call, never fatal to the import.
pub fn refresh() -> serde_json::Value {
    let count = |sql: &str| match host::execute_raw(sql, &[]) {
        Ok(n) => i64::try_from(n).unwrap_or(i64::MAX),
        Err(_) => -1,
    };
    serde_json::json!({
        "titles_mirrored": count(MIRROR_TITLE),
        "topic_names_written": count(DENORMALIZE_TOPICS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_statements_only_touch_rows_whose_value_differs() {
        // The guard is what makes running this on every cron call cheap and
        // keeps `changed` from moving (and Pagefind from rebuilding) for nothing.
        assert!(MIRROR_TITLE.contains("IS DISTINCT FROM title"));
        assert!(DENORMALIZE_TOPICS.contains("IS DISTINCT FROM w.topic_names"));
    }

    #[test]
    fn refresh_reports_both_counts() {
        let out = refresh();
        assert!(out.get("titles_mirrored").is_some());
        assert!(out.get("topic_names_written").is_some());
    }
}
