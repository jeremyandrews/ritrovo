//! Subscription and notification plugin for Ritrovo conferences: not built yet.
//!
//! This plugin is a placeholder that installs, enables and does nothing. What it
//! is meant to do is in `README.md` beside this file.
//!
//! It used to declare a `/user/subscriptions` menu entry with no `tap_api` behind
//! it, so the path 404ed; render a Subscribe button on every conference page with
//! no form or endpoint behind it, so the button did nothing; declare a
//! `ritrovo_notifications` queue with no `tap_queue_worker` to drain it; and
//! register two permissions that nothing checked. A route that 404s and a button
//! that does nothing are worse than their absence, and so is a queue that only
//! fills and a permission an administrator can grant to no effect, so all four are
//! gone until the taps that make them true exist.
//!
//! What stays is the `pending_notifications` table its migration creates, which
//! the real implementation will write, and which is already applied on every site
//! that has enabled this plugin.
