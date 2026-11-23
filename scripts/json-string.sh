# shellcheck shell=sh
#
# json_string VALUE: print VALUE as a JSON string, quotes included.
#
# Sourced by demo-bootstrap.sh (POSIX sh, in the kernel image) and by
# verify-demo.sh (bash, on the host), which both post a login as JSON. A value
# put between quotes as it stands turns into broken JSON, or into a different
# document, when it holds a quote or a backslash, and DEMO_ADMIN_PASSWORD is
# the operator's to choose. So backslash and quote are escaped, and tab,
# carriage return and newline are written as their escapes. Other control
# characters are not escaped; a password holding one is refused by the kernel's
# JSON parser rather than misread.
#
# Covered by demo/checks/tests/demo_users.rs.

json_string() {
    _json_tab="$(printf '\t')"
    _json_cr="$(printf '\r')"
    # The whole value is gathered into one pattern space first, so every
    # substitution sees every line.
    printf '"%s"' "$(printf '%s' "$1" | sed \
        -e ':a' -e '$!N' -e '$!ba' \
        -e 's/\\/\\\\/g' \
        -e 's/"/\\"/g' \
        -e "s/$_json_tab/\\\\t/g" \
        -e "s/$_json_cr/\\\\r/g" \
        -e 's/\n/\\n/g')"
}
