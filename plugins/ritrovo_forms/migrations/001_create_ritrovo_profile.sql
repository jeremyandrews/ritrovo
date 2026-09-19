-- The member profile fields Ritrovo adds to a Trovato user.
--
-- Forward-only migration; no rollback.
--
-- WHY THIS TABLE EXISTS. The kernel's profile form at /user/profile is a fixed
-- struct — username, email, timezone, password — extracted with
-- `Form<ProfileFormRequest>`, and no tap is dispatched anywhere along it. There
-- is no seam through which a plugin adds a field to a user, so a bio cannot live
-- on the user record. See FRICTION.md, G-USER-PROFILE-NOT-EXTENSIBLE.
--
-- One row per user who has written something. A user with no row has no bio,
-- which is the same thing an empty bio means, so nothing seeds this table.
--
-- No foreign key to `users`. The kernel deletes a user by reattributing their
-- content to the anonymous author rather than cascading, and a plugin's table is
-- not in that transaction; a REFERENCES clause here would make a user deletion
-- fail on a table the kernel has never heard of. `orphaned` rows are instead
-- harmless: nothing reads a row whose user cannot be loaded.
CREATE TABLE IF NOT EXISTS ritrovo_profile (
    user_id UUID PRIMARY KEY,
    bio TEXT NOT NULL DEFAULT '',
    updated BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::bigint
);
