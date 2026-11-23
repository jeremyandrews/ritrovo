# shellcheck shell=sh
#
# The demo's three editorial accounts, for demo-bootstrap.sh to source.
#
# editor_alice, publisher_bob and viewer_carol are the accounts the walkthrough
# in docs/EDITORIAL.md logs in as. demo_users_create makes each one that does
# not exist yet, through the administrator's "Add user" form, while the
# bootstrap server is up. demo_users_grant_roles gives each its role with
# `trovato user role-add`, after the config import has created the roles.
#
# Only an account this run created is given a role. The bootstrap runs on every
# start and registration is open, so an account under one of these names that
# the bootstrap did not just make may be anyone's: someone who registered
# publisher_bob on a demo started without that account would otherwise be made
# a publisher at the next restart. The cost is one case that fails closed: a
# start that dies between creating an account and granting its role leaves that
# account without one, and a later start leaves it so. `serve-demo.sh --fresh`,
# or `trovato user role-add` by hand, mends it.
#
# Needs: BASE, DEMO_ADMIN_USER, DEMO_ADMIN_PASSWORD, json_string (from
# json-string.sh), curl and sed. TROVATO is the kernel binary, ./trovato by
# default. Covered by demo/checks/tests/demo_users.rs, which runs both
# functions against a stub kernel and a stub curl.

# name:role:password, for each account.
DEMO_USERS="editor_alice:editor:ritrovo-editor-demo publisher_bob:publisher:ritrovo-publisher-demo viewer_carol:viewer:ritrovo-viewer-demo"

# The names demo_users_create made in this run, space separated.
DEMO_USERS_CREATED=""

# Create each demo account that does not exist. Returns non-zero, having said
# which, when the kernel did not create one.
demo_users_create() {
    _trovato="${TROVATO:-./trovato}"
    _jar="$(mktemp)"
    _logged_in=0
    for _entry in $DEMO_USERS; do
        _name="${_entry%%:*}"
        _password="${_entry##*:}"
        if "$_trovato" user roles "$_name" >/dev/null 2>&1; then
            continue
        fi
        if [ "$_logged_in" -eq 0 ]; then
            curl -fsS -o /dev/null -c "$_jar" -X POST "$BASE/user/login/json" \
                -H 'Content-Type: application/json' \
                --data-binary "{\"username\":$(json_string "$DEMO_ADMIN_USER"),\"password\":$(json_string "$DEMO_ADMIN_PASSWORD")}"
            _logged_in=1
        fi
        echo "==> creating the demo account $_name"
        _form="$(curl -fsS -b "$_jar" -c "$_jar" "$BASE/admin/people/add")"
        _token="$(printf '%s' "$_form" | sed -n 's/.*name="_token" value="\([^"]*\)".*/\1/p' | head -n 1)"
        _build="$(printf '%s' "$_form" | sed -n 's/.*name="_form_build_id" value="\([^"]*\)".*/\1/p' | head -n 1)"
        curl -fsS -o /dev/null -b "$_jar" -c "$_jar" -X POST "$BASE/admin/people/add" \
            --data-urlencode "_token=$_token" \
            --data-urlencode "_form_build_id=$_build" \
            --data-urlencode "name=$_name" \
            --data-urlencode "mail=$_name@ritrovo.example" \
            --data-urlencode "password=$_password" \
            --data-urlencode "status=1"
        if ! "$_trovato" user roles "$_name" >/dev/null 2>&1; then
            echo "error: the kernel did not create $_name" >&2
            rm -f "$_jar"
            return 1
        fi
        DEMO_USERS_CREATED="$DEMO_USERS_CREATED $_name"
    done
    rm -f "$_jar"
}

# Give each account demo_users_create made in this run its role, and no other.
demo_users_grant_roles() {
    _trovato="${TROVATO:-./trovato}"
    for _entry in $DEMO_USERS; do
        _name="${_entry%%:*}"
        _role="${_entry#*:}"
        _role="${_role%%:*}"
        case " $DEMO_USERS_CREATED " in
            *" $_name "*) "$_trovato" user role-add "$_name" "$_role" ;;
            *) echo "==> $_name was not created by this start; its roles are left as they are" ;;
        esac
    done
}
