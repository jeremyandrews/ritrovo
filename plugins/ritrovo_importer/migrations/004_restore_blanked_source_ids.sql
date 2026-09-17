-- Restore the dedup key on conferences whose update blanked it, and merge the
-- duplicates that blanking let in.
--
-- Root cause: `update_conference` rebuilt the source fields with an empty
-- `field_source_id` and merged them into the row, so the first update of any
-- conference (the first cron run after install, on every site) set its key to
-- ''. `load_existing_conferences` skips empty keys and the unique index
-- `uniq_item_conference_source_id` excludes them, so the next import of that
-- conference inserted a second Item instead of updating the first, and so on for
-- every run after. The plugin now writes the key back; this repairs the rows.
--
-- The key is recomputed the way `compute_source_id` computes it:
--   slug(name) || '-' || startDate || '-' || slug(city), or 'online' with no city
-- where slug keeps ASCII letters and digits, lowercases them, and turns every
-- other run of characters into one hyphen, trimmed from both ends. The update
-- writes the title from the source name, so the title is the name. The character
-- class is ASCII and applied before lower(), so a locale that lowercases a
-- non-ASCII letter into ASCII cannot make SQL and Rust disagree.
--
-- Safe to replay: with no empty keys left, every step finds nothing to do. The
-- scratch table is dropped explicitly rather than ON COMMIT, so the file also
-- runs statement by statement outside a transaction (psql -f).

DROP TABLE IF EXISTS ritrovo_source_ids;
CREATE TEMP TABLE ritrovo_source_ids AS
SELECT
    id,
    created,
    CASE
        WHEN COALESCE(fields->>'field_source_id', '') <> '' THEN fields->>'field_source_id'
        ELSE lower(trim(both '-' from regexp_replace(title, '[^A-Za-z0-9]+', '-', 'g')))
             || '-' || (fields->>'field_start_date') || '-'
             || COALESCE(
                    lower(trim(both '-' from
                        regexp_replace(fields->>'field_city', '[^A-Za-z0-9]+', '-', 'g'))),
                    'online')
    END AS source_id
FROM item
WHERE type = 'conference'
  AND fields->>'field_start_date' IS NOT NULL;

-- Step 1: fold every copy's topics into the earliest copy of each conference.
WITH canonical AS (
    SELECT DISTINCT ON (source_id) id, source_id
    FROM ritrovo_source_ids
    ORDER BY source_id, created ASC, id ASC
),
merged AS (
    SELECT
        c.id AS canonical_id,
        (
            SELECT COALESCE(jsonb_agg(t ORDER BY t), '[]'::jsonb)
            FROM (
                SELECT jsonb_array_elements_text(i.fields->'field_topics')
                FROM item i
                JOIN ritrovo_source_ids s ON s.id = i.id
                WHERE s.source_id = c.source_id
                GROUP BY 1
            ) u(t)
        ) AS topics
    FROM canonical c
    WHERE (SELECT count(*) FROM ritrovo_source_ids s WHERE s.source_id = c.source_id) > 1
)
UPDATE item
SET fields = fields || jsonb_build_object('field_topics', merged.topics)
FROM merged
WHERE item.id = merged.canonical_id;

-- Step 2: delete the later copies.
DELETE FROM item
WHERE id IN (
    SELECT id FROM (
        SELECT id, ROW_NUMBER() OVER (PARTITION BY source_id ORDER BY created ASC, id ASC) AS rn
        FROM ritrovo_source_ids
    ) ranked
    WHERE rn > 1
);

-- Step 3: write the key back onto the survivors that lost it. Last, so no row
-- takes a key another row still holds.
UPDATE item
SET fields = item.fields || jsonb_build_object('field_source_id', s.source_id)
FROM ritrovo_source_ids s
WHERE item.id = s.id
  AND COALESCE(item.fields->>'field_source_id', '') = '';

DROP TABLE ritrovo_source_ids;
