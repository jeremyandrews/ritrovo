-- Server-side state for a multi-step form, between one step and the next.
--
-- Forward-only migration; no rollback.
--
-- WHY THIS TABLE EXISTS, WHEN THE KERNEL HAS ONE. The kernel ships
-- `form_state_cache`, created by its own migration, with exactly the columns a
-- multi-step form needs and a cron task that expires stale rows. `FormService`
-- writes it. **No route calls `FormService`**, and no host interface exposes it,
-- so a plugin cannot read or write that table through anything but raw SQL
-- against a table it does not own. See FRICTION.md, G-FORM-TAPS-UNREACHABLE and
-- G-NO-FORM-STATE-HOST-API.
--
-- Writing to the kernel's table anyway was the other option and is worse: the
-- schema is the kernel's to change, the cron task is the kernel's to retune, and
-- a plugin squatting in it would break on a release that did either. So this is
-- the same shape under a name this plugin owns, and the recommendation in
-- FRICTION.md is that the kernel should own this rather than each plugin
-- reinventing it.
--
-- WHY THE STATE IS HERE AND NOT IN THE BROWSER. A hidden field carrying the
-- accumulated answers would need no table, and would let anyone submit any
-- conference with any submitter by editing it. The row is the authority; the
-- browser carries only the draft's id, which is checked against `user_id` on
-- every read.
CREATE TABLE IF NOT EXISTS ritrovo_form_state (
    id UUID PRIMARY KEY,
    -- Whose draft this is. Every read is filtered on it, so one member's draft
    -- id is useless to another member.
    user_id UUID NOT NULL,
    -- Which form. One table serves every multi-step form this plugin grows.
    form_id TEXT NOT NULL,
    -- The highest step whose answers are in `state`. A step cannot be skipped,
    -- and this is what says so.
    step SMALLINT NOT NULL DEFAULT 0,
    -- The answers so far, one key per form field.
    state JSONB NOT NULL DEFAULT '{}',
    created BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::bigint,
    updated BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::bigint
);

-- For expiry. A draft is abandoned far more often than it is finished, so
-- without this the table only grows.
CREATE INDEX IF NOT EXISTS idx_ritrovo_form_state_updated
    ON ritrovo_form_state (updated);

-- For "my drafts", and for the per-user filter every read applies.
CREATE INDEX IF NOT EXISTS idx_ritrovo_form_state_user
    ON ritrovo_form_state (user_id, form_id);
