-- Persistent key-value state for the ritrovo_importer plugin.
-- Used to track import timestamps, topic offsets, and ETags.
--
-- IF NOT EXISTS so replaying this file against an installed site is a no-op
-- rather than an error. The kernel records applied migrations by file name and
-- never reruns one, so this changes nothing for an install that already has it.

CREATE TABLE IF NOT EXISTS ritrovo_state (
    name VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL
);
