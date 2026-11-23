//! The demo's editorial accounts, as `scripts/demo-users.sh` makes them.
//!
//! The functions `demo-bootstrap.sh` sources are run here under `sh`, the
//! shell the bootstrap runs in, against a stub `trovato` and a stub `curl` on
//! `PATH`. The stubs record what the script asked for in a directory of their
//! own under the system temp directory, which each test removes again.
//!
//! Two things are held: only an account the script created in this run is
//! given a role, so a name someone registered first is never promoted; and the
//! administrator's login is posted as JSON that reads back as the exact user
//! name and password, whatever characters those hold.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use ritrovo_demo_checks::repo_root;

/// Answers `user roles NAME` with success when NAME has a file under
/// `users/`, and records every `user role-add NAME ROLE`.
const TROVATO_STUB: &str = r#"#!/bin/sh
case "$1 $2" in
    "user roles") [ -e "$STUB_DIR/users/$3" ] ;;
    "user role-add") echo "$3 $4" >> "$STUB_DIR/role-adds" ;;
    *) echo "unexpected: $*" >&2; exit 2 ;;
esac
"#;

/// Serves the "Add user" form on a GET, creates the account named in a POST
/// to it, and keeps the JSON body of the login.
const CURL_STUB: &str = r#"#!/bin/sh
url=""; name=""
while [ $# -gt 0 ]; do
    case "$1" in
        --data-binary) shift; printf '%s' "$1" > "$STUB_DIR/login.json" ;;
        --data-urlencode) shift; case "$1" in name=*) name="${1#name=}" ;; esac ;;
        -o|-c|-b|-X|-H|-w) shift ;;
        http*) url="$1" ;;
    esac
    shift
done
case "$url" in
    */admin/people/add)
        if [ -n "$name" ]; then
            : > "$STUB_DIR/users/$name"
        else
            printf '<input name="_token" value="t"><input name="_form_build_id" value="b">'
        fi ;;
esac
"#;

/// A stub directory under the system temp directory, removed on drop.
struct Stubs(PathBuf);

impl Stubs {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("ritrovo-demo-users-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::create_dir_all(dir.join("users")).unwrap();
        for (name, body) in [("trovato", TROVATO_STUB), ("curl", CURL_STUB)] {
            let path = dir.join("bin").join(name);
            fs::write(&path, body).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(dir)
    }

    /// An account that exists before the script runs, made by someone else.
    fn existing(&self, name: &str) {
        fs::write(self.0.join("users").join(name), "").unwrap();
    }

    fn read(&self, file: &str) -> String {
        fs::read_to_string(self.0.join(file)).unwrap_or_default()
    }

    /// Source the two scripts as the bootstrap does and run both functions.
    fn run(&self, admin_user: &str, admin_password: &str) {
        let bin = self.0.join("bin");
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let output = Command::new("sh")
            .arg("-c")
            .arg(
                ". scripts/json-string.sh && . scripts/demo-users.sh \
                 && demo_users_create && demo_users_grant_roles",
            )
            .current_dir(repo_root())
            .env("PATH", path)
            .env("STUB_DIR", &self.0)
            .env("TROVATO", bin.join("trovato"))
            .env("BASE", "http://stub.invalid")
            .env("DEMO_ADMIN_USER", admin_user)
            .env("DEMO_ADMIN_PASSWORD", admin_password)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "demo-users.sh failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for Stubs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn exists(dir: &Path, name: &str) -> bool {
    dir.join("users").join(name).exists()
}

#[test]
fn only_an_account_this_start_created_is_given_a_role() {
    let stubs = Stubs::new("roles");
    // Someone registered publisher_bob before the bootstrap made it.
    stubs.existing("publisher_bob");

    stubs.run("admin", "ritrovo-demo-password");

    assert!(exists(&stubs.0, "editor_alice"));
    assert!(exists(&stubs.0, "viewer_carol"));
    assert_eq!(
        stubs.read("role-adds"),
        "editor_alice editor\nviewer_carol viewer\n",
        "a role went to an account the bootstrap did not create"
    );

    // The next start finds all three and grants nothing.
    fs::remove_file(stubs.0.join("role-adds")).unwrap();
    stubs.run("admin", "ritrovo-demo-password");
    assert_eq!(stubs.read("role-adds"), "");
}

#[test]
fn the_admin_login_is_json_that_reads_back_exactly() {
    let stubs = Stubs::new("login");
    let user = r#"ad"min\"#;
    let password = "pa\"ss\\word\twith \"quotes\", a tab\nand a line";

    stubs.run(user, password);

    let login: serde_json::Value = serde_json::from_str(&stubs.read("login.json"))
        .unwrap_or_else(|e| panic!("the login is not JSON ({e}): {}", stubs.read("login.json")));
    assert_eq!(
        login,
        serde_json::json!({ "username": user, "password": password })
    );
}
