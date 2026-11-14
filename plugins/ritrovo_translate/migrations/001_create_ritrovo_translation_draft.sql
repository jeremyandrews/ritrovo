-- Model-drafted translations awaiting an editor's review.
--
-- NOT a translation table. A row here is never rendered: the queue page at
-- /admin/content/translations shows it to an editor beside the link to the
-- kernel's translate screen, and the editor saves the reviewed text there, into
-- the kernel's own item_translation. This table exists because no host call
-- lets a plugin write item_translation (FRICTION.md, G-TRANSLATION-NO-WRITE-PATH),
-- so a draft needs somewhere to wait that is plainly not the real thing.
--
-- Filled only when the plugin variable assist_enabled is 1, which it is not by
-- default. IF NOT EXISTS so a replay is a no-op.

CREATE TABLE IF NOT EXISTS ritrovo_translation_draft (
    item_id  UUID        NOT NULL,
    language VARCHAR(12) NOT NULL,
    title    TEXT        NOT NULL,
    fields   JSONB       NOT NULL DEFAULT '{}',
    model    TEXT        NOT NULL DEFAULT '',
    created  BIGINT      NOT NULL,
    PRIMARY KEY (item_id, language)
);
