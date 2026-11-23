#!/usr/bin/env bash
#
# Print the state of plugin NAME, read from `trovato plugin list` on stdin.
#
# Usage: trovato plugin list | scripts/plugin-state.sh NAME
#
# Prints one of:
#   enabled, disabled, not installed   the status column of NAME's row
#   absent                             a plugin list that does not name NAME
#   unknown                            no plugin list at all
#
# The status is the third column, matched whole: "disabled" contains
# "enabled", so a substring test would pass a plugin that is switched off.
# Only "unknown" means the list could not be read (verify-demo.sh run without
# the Compose stack); a list that leaves NAME out is "absent".
#
# Covered by demo/checks/tests/plugin_state.rs.

set -euo pipefail

[ $# -eq 1 ] && [ -n "$1" ] || { echo "usage: $0 NAME" >&2; exit 2; }

awk -v name="$1" '
    $1 == "PLUGIN" && $3 == "STATUS" { listed = 1; next }
    $0 == "No plugins found."        { listed = 1; next }
    listed && $1 == name             { state = ($3 == "not" && $4 == "installed") ? "not installed" : $3 }
    END {
        if (state != "")  print state
        else if (listed)  print "absent"
        else              print "unknown"
    }
'
