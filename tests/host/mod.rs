//! The host every Ritrovo host-in-the-loop suite runs its plugin on.
//!
//! Each `plugins/<name>/tests/<name>_host.rs` includes this file with
//! `#[path = "../../../tests/host/mod.rs"] mod host;`. It is not a crate: the
//! suites need exactly one kernel dev-dependency each, pinned to the SDK's rev in
//! the workspace manifest, and a shared crate would be one more workspace member
//! that `cargo build --target wasm32-wasip1` would try, and fail, to compile.
//!
//! What it provides is the kernel, not a model of it:
//!
//! - the **compiled** module, loaded from the assembled overlay
//!   (`scripts/assemble-overlay.sh`), never from a copy the test keeps;
//! - the kernel's own install, enable, API-version and migration code
//!   (`trovato_kernel::plugin::cli`, `plugin::migration`);
//! - the real `TapDispatcher`, `ItemService` and `CronService`;
//! - a real Postgres, and for the one suite that serves HTTP, a real `AppState`
//!   with its Redis-backed sessions;
//! - an HTTPS fixture server for anything a plugin fetches, so no test ever
//!   reaches the network (see [`FixtureServer`]).
//!
//! Every suite runs its tests one at a time ([`serial`]): they share one database,
//! and a test that truncates the queue under another test's drain is a flake.
//! Cargo runs the six test binaries one after another, never in parallel, so the
//! database is only ever in one suite's hands at a time.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::Duration;

use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use trovato_kernel::content::ItemService;
use trovato_kernel::plugin::{PluginConfig, PluginRuntime};
use trovato_kernel::tap::{RequestServices, RequestState, TapDispatcher, TapRegistry, UserContext};

/// The two internal editorial stages, as `demo/config/stage.*.yml` identifies
/// them. See [`import_demo_config`].
pub const INCOMING_STAGE: &str = "0193a5a0-0000-7000-8000-000000000002";
pub const CURATED_STAGE: &str = "0193a5a0-0000-7000-8000-000000000003";

// ===========================================================================
// Runtime and database
// ===========================================================================

static SERIAL: Mutex<()> = Mutex::new(());

/// One runtime for the whole binary. Pool connections and the `AppState`'s Redis
/// connections are bound to the runtime that opened them, so a runtime per test
/// would leave the next test holding connections to a dead reactor.
static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build test runtime")
});

/// Run one test body, alone, on the shared runtime.
pub fn serial<F: std::future::Future<Output = ()>>(body: F) {
    let _guard = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    RT.block_on(body);
}

pub fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://trovato:trovato@localhost:5432/trovato".to_string())
}

pub fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// A pool on a database the kernel's own migrations have been run against.
///
/// No fallback and no skip: without a database this panics, and CI asserts the
/// suites ran, so a missing service is a red build rather than a quiet one.
pub async fn fresh_pool() -> PgPool {
    let url = database_url();
    let pool = PgPool::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("connect to {url}: {e}. These tests need Postgres."));
    trovato_kernel::db::run_migrations(&pool)
        .await
        .expect("run kernel migrations");
    pool
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs() as i64
}

/// Any user id, for a column that only has to satisfy a foreign key.
pub async fn any_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("SELECT id FROM users ORDER BY created LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("the kernel migrations seed at least one user")
}

// ===========================================================================
// The overlay and the plugin lifecycle
// ===========================================================================

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR is <root>/plugins/<name>")
}

/// The search path entry a deployment appends to `PLUGINS_DIR`.
///
/// The overlay rather than `plugins/<name>/`, because that is where the module
/// and its manifest sit side by side, which is what the kernel loads. It is also
/// what makes these tests meet the module CI just built: nothing here keeps a
/// `.wasm` of its own that could go stale.
pub fn overlay_plugins() -> PathBuf {
    repo_root().join("overlay/plugins")
}

pub fn plugin_dir(name: &str) -> PathBuf {
    let dir = overlay_plugins().join(name);
    assert!(
        dir.join(format!("{name}.wasm")).is_file(),
        "{} has no {name}.wasm; build and assemble the overlay first:\n  \
         cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh",
        dir.display()
    );
    dir
}

/// Forget everything the kernel knows about a plugin being installed, so the
/// next install is a first install. Its own tables are the suite's to reset.
pub async fn uninstall(pool: &PgPool, name: &str) {
    for sql in [
        "DELETE FROM plugin_status WHERE name = $1",
        "DELETE FROM plugin_migration WHERE plugin = $1",
    ] {
        sqlx::query(sql).bind(name).execute(pool).await.unwrap();
    }
}

/// `trovato plugin install <name>` then `trovato plugin enable <name>`, through
/// the kernel's own CLI functions, against the overlay.
///
/// Install is where the kernel checks `api_version`, checks dependencies and runs
/// the manifest's migrations; enable checks `api_version` again. Both are the
/// code an operator's command runs, so a manifest that an operator could not
/// install fails here the same way.
pub async fn install_and_enable(pool: &PgPool, name: &str) -> Result<(), String> {
    let dirs = [overlay_plugins()];
    trovato_kernel::plugin::cli::cmd_plugin_install(pool, &dirs, name)
        .await
        .map_err(|e| format!("{e:#}"))?;
    trovato_kernel::plugin::cli::cmd_plugin_enable(pool, &dirs, name)
        .await
        .map_err(|e| format!("{e:#}"))
}

pub async fn plugin_status(pool: &PgPool, name: &str) -> Option<i16> {
    sqlx::query_scalar("SELECT status FROM plugin_status WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
        .unwrap()
}

/// Migration file names the kernel has recorded as applied for a plugin.
pub async fn applied_migrations(pool: &PgPool, name: &str) -> Vec<String> {
    trovato_kernel::plugin::migration::get_applied_migrations(pool, name)
        .await
        .unwrap()
}

/// Run the kernel's migration runner for one plugin, as the server does at start.
pub async fn run_migrations(pool: &PgPool, name: &str) -> Vec<String> {
    let dir = plugin_dir(name);
    let info = trovato_kernel::plugin::PluginInfo::parse(&dir.join(format!("{name}.info.toml")))
        .expect("parse manifest");
    trovato_kernel::plugin::migration::run_plugin_migrations(pool, name, &info, &dir)
        .await
        .unwrap_or_else(|e| panic!("migrations for {name}: {e:#}"))
}

/// Execute every migration file again, directly, in manifest order.
///
/// The runner never reruns a recorded migration, so this is not something the
/// kernel does. It is what an operator does when they replay a file by hand, and
/// a migration that punishes that is one that also breaks on a half-applied
/// install.
pub async fn replay_migrations_raw(pool: &PgPool, name: &str) {
    let dir = plugin_dir(name);
    let info = trovato_kernel::plugin::PluginInfo::parse(&dir.join(format!("{name}.info.toml")))
        .expect("parse manifest");
    for file in &info.migrations.files {
        let sql = std::fs::read_to_string(dir.join(file)).unwrap();
        sqlx::raw_sql(&sql)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("replaying {name}/{file} failed: {e}"));
    }
}

/// A dispatcher over exactly one compiled plugin, loaded from the overlay.
///
/// One per binary: each wasmtime runtime reserves a large slab of address space,
/// and a runtime per test would exhaust it long before it exhausted anything else.
pub fn dispatcher(name: &'static str) -> Arc<TapDispatcher> {
    static DISPATCHER: OnceLock<(&'static str, Arc<TapDispatcher>)> = OnceLock::new();
    let (loaded, disp) = DISPATCHER.get_or_init(|| {
        let mut runtime = PluginRuntime::new(&PluginConfig::default()).expect("create runtime");
        runtime
            .load_plugin(&plugin_dir(name))
            .unwrap_or_else(|e| panic!("the kernel refused to load {name}: {e:#}"));
        let runtime = Arc::new(runtime);
        let registry = Arc::new(TapRegistry::from_plugins(&runtime));
        (name, Arc::new(TapDispatcher::new(runtime, registry)))
    });
    assert_eq!(*loaded, name, "one plugin per test binary");
    disp.clone()
}

/// Request services with the given outbound client. The client is the test's
/// choice, which is what lets [`FixtureServer`] stand in for the internet.
pub fn services(
    pool: &PgPool,
    disp: &Arc<TapDispatcher>,
    http: reqwest::Client,
) -> RequestServices {
    RequestServices::for_background(pool.clone(), None, None, http)
        .with_plugin_runtime(disp.runtime().clone())
}

/// An outbound client that cannot reach anything: every connection is refused.
///
/// The default for taps that have no business making a request. `reqwest`'s own
/// default client would go to the real network if a plugin tried.
pub fn offline_client() -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:9").expect("proxy url"))
        .build()
        .expect("build offline client")
}

pub fn state_as(
    pool: &PgPool,
    disp: &Arc<TapDispatcher>,
    user: UserContext,
    http: reqwest::Client,
) -> RequestState {
    RequestState::new(user, services(pool, disp, http))
}

/// Dispatch a tap to this binary's plugin and return its raw output.
pub async fn dispatch(
    pool: &PgPool,
    disp: &Arc<TapDispatcher>,
    plugin: &str,
    tap: &str,
    input: &str,
    user: UserContext,
    http: reqwest::Client,
) -> String {
    disp.dispatch_to_plugin(tap, input, plugin, state_as(pool, disp, user, http))
        .await
        .unwrap_or_else(|| panic!("{plugin} did not answer {tap}"))
        .output
}

/// An `ItemService` wired to the plugin's dispatcher, so creating, viewing and
/// access-checking an Item fires the plugin's taps exactly as the routes do.
pub fn items(pool: &PgPool, disp: &Arc<TapDispatcher>) -> ItemService {
    ItemService::new(
        pool.clone(),
        disp.clone(),
        services(pool, disp, offline_client()),
        Duration::from_secs(60),
        None,
        None,
    )
}

/// Import Ritrovo's configuration set, through the kernel's own config importer.
///
/// `demo/config/` is the set itself, not a copy of one: the `topics` category and
/// its terms (which `ritrovo_importer` resolves by label), the `conference` and
/// `speaker` types (which every Item's `type` is a foreign key onto), the four
/// editorial stages (which `ritrovo_access` gates on), the gathers, roles, tiles,
/// menu links and aliases. The demo imports this same directory.
///
/// It used to be `tests/host/tutorial-config/`, a hand-picked subset of the
/// kernel image's copy that CI diffed against the release. That existed because
/// the model was Trovato's; it is Ritrovo's now, so the tests read the real thing
/// and there is no subset to drift.
///
/// The whole set imports or none of it does — references resolve across the
/// directory and then the database, and one unresolved reference fails the import
/// with nothing written — so a test that needs any of it gets all of it.
///
/// Idempotent: the importer upserts.
pub async fn import_demo_config(pool: &PgPool) {
    let storage = trovato_kernel::config_storage::DirectConfigStorage::new(pool.clone());
    let dir = repo_root().join("demo/config");
    trovato_kernel::config_storage::yaml::import_config(&storage, pool, &dir, false)
        .await
        .unwrap_or_else(|e| panic!("import {}: {e:#}", dir.display()));
}

/// The id of a `topics` term, by the label `demo/config` gives it.
pub async fn topic_term(pool: &PgPool, label: &str) -> String {
    sqlx::query_scalar(
        "SELECT id::text FROM category_tag WHERE category_id = 'topics' AND label = $1",
    )
    .bind(label)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("no topics term labelled {label}: {e}"))
}

// ===========================================================================
// HTTPS fixtures
// ===========================================================================

/// An HTTPS server on loopback that answers as a named public host, from files.
///
/// The kernel's `http` host refuses loopback and private addresses **by URL**
/// before it sends anything, and resolves hostnames through a resolver that
/// refuses private answers. Neither can be switched off, and neither should be:
/// they are the SSRF fence. What the kernel does not choose is the
/// `reqwest::Client` a test hands `RequestServices`. So the plugin keeps asking
/// for its real `https://raw.githubusercontent.com/...` URL, the URL passes the
/// fence because it is a public name, and the test's client resolves that one
/// name to this server, which presents a certificate for it signed by a CA only
/// that client trusts.
///
/// The plugin is not modified, not configured, and cannot tell. Any other host
/// it asked for would resolve normally, which is why [`Self::requests`] records
/// every path served: a suite asserts the plugin asked for nothing it did not
/// expect.
pub struct FixtureServer {
    host: String,
    addr: SocketAddr,
    ca_pem: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl FixtureServer {
    /// Serve `root/<url path minus prefix>` for `https://<host><prefix>/...`.
    /// Anything without a file answers 404, which is what GitHub answers too.
    pub async fn start(host: &str, prefix: &str, root: PathBuf) -> Self {
        use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};

        let ca_key = KeyPair::generate().unwrap();
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "Ritrovo test fixtures CA");
        let ca = ca_params.self_signed(&ca_key).unwrap();

        let leaf_key = KeyPair::generate().unwrap();
        let leaf_params = CertificateParams::new(vec![host.to_string()]).unwrap();
        let leaf = leaf_params.signed_by(&leaf_key, &ca, &ca_key).unwrap();

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![leaf.der().clone()],
                rustls::pki_types::PrivateKeyDer::Pkcs8(leaf_key.serialize_der().into()),
            )
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));

        let prefix = prefix.to_string();
        let log = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    return;
                };
                let acceptor = acceptor.clone();
                let root = root.clone();
                let prefix = prefix.clone();
                let log = log.clone();
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        match tls.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let head = String::from_utf8_lossy(&buf).to_string();
                    let path = head
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_string();
                    log.lock().unwrap().push(path.clone());

                    let file = path
                        .strip_prefix(&prefix)
                        .filter(|rest| !rest.contains(".."))
                        .map(|rest| root.join(rest.trim_start_matches('/')));
                    let response = match file.and_then(|f| std::fs::read(f).ok()) {
                        Some(body) => {
                            let etag = format!("\"fixture-{}\"", body.len());
                            let mut r = format!(
                                "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\n\
                                 etag: {etag}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                                body.len()
                            )
                            .into_bytes();
                            r.extend_from_slice(&body);
                            r
                        }
                        None => b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                            .to_vec(),
                    };
                    let _ = tls.write_all(&response).await;
                    let _ = tls.shutdown().await;
                });
            }
        });

        Self {
            host: host.to_string(),
            addr,
            ca_pem: ca.pem(),
            requests,
        }
    }

    /// The client a plugin's requests should go out on: the one public host
    /// resolved to this server, and this server's CA trusted.
    pub fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .resolve(&self.host, self.addr)
            .add_root_certificate(reqwest::Certificate::from_pem(self.ca_pem.as_bytes()).unwrap())
            .build()
            .expect("build fixture client")
    }

    /// Every request path served so far, in arrival order.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

// ===========================================================================
// The HTTP app, for the suites that assert on routes
// ===========================================================================

pub struct App {
    pub state: trovato_kernel::AppState,
    router: axum::Router,
}

/// The kernel's `AppState` over the overlay, and a router carrying the routes
/// these suites request: login, and every plugin-served API route.
///
/// Built once per binary, on the shared runtime, for the same address-space
/// reason as [`dispatcher`]. The middleware is the subset of the kernel's
/// production stack these routes read from: the peer address `resolve_client_ip`
/// needs, the client IP the login rate limiter needs, and the Redis session.
///
/// **Call [`prepare_app_database`] first.** `AppState::new` dispatches
/// `tap_install` to any enabled plugin that has not had it, on the kernel's own
/// outbound client, which reaches the real network.
pub async fn app() -> &'static App {
    static APP: OnceLock<App> = OnceLock::new();
    if let Some(app) = APP.get() {
        return app;
    }
    let mut config = trovato_kernel::Config::from_env().expect("load kernel config");
    config.database_url = database_url();
    config.redis_url = redis_url();
    config.plugins_dirs = vec![overlay_plugins()];
    config.trusted_proxies = vec![std::net::IpAddr::from([127, 0, 0, 1])];

    let state = trovato_kernel::AppState::new(&config)
        .await
        .unwrap_or_else(|e| panic!("AppState::new: {e:#}. These tests need Postgres and Redis."));

    let session_layer = trovato_kernel::session::create_session_layer(
        &config.redis_url,
        tower_sessions::cookie::SameSite::Strict,
    )
    .await
    .expect("create session layer");

    let router = axum::Router::new()
        .merge(trovato_kernel::routes::auth::router())
        // The kernel's own item routes, `/item/{id}/edit` among them. A6 tests
        // what that form preserves and what it destroys, and the only honest way
        // to find out is to drive the kernel's route rather than a reading of it.
        .merge(trovato_kernel::routes::item::router())
        .merge(trovato_kernel::routes::plugin_api::build_plugin_api_router(
            &state.menu_registry().all().cloned().collect::<Vec<_>>(),
        ))
        .layer(session_layer)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            trovato_kernel::middleware::resolve_client_ip,
        ))
        .layer(axum::middleware::from_fn(
            |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
                req.extensions_mut()
                    .insert(axum::extract::ConnectInfo(SocketAddr::from((
                        [127, 0, 0, 1],
                        0,
                    ))));
                next.run(req).await
            },
        ))
        .with_state(state.clone());

    let _ = APP.set(App { state, router });
    APP.get().expect("app just set")
}

/// Install and enable the named plugins and record their `tap_install` as done,
/// so building the `AppState` loads them without firing any install tap.
pub async fn prepare_app_database(pool: &PgPool, plugins: &[&str]) {
    // Disable every Ritrovo plugin this app does not ask for, FIRST.
    //
    // `app()` builds a real AppState, which runs the migrations of whatever the
    // database says is enabled. The database is shared by every test binary and
    // cargo runs those binaries in parallel, so "whatever is enabled" includes
    // anything another suite switched on and has not switched off. That bites on
    // one plugin in particular: ritrovo_translate declares a dependency on the
    // kernel's trovato_content_translation, which is not in the overlay these
    // tests load, so an AppState built while translate happens to be enabled
    // fails with "depends on 'trovato_content_translation' which is not in the
    // enabled plugin set" — in a suite that has nothing to do with translation.
    //
    // Naming the set positively rather than hoping the database is clean.
    for name in [
        "ritrovo_importer",
        "ritrovo_access",
        "ritrovo_cfp",
        "ritrovo_notify",
        "ritrovo_translate",
    ] {
        if !plugins.contains(&name) {
            let _ = trovato_kernel::plugin::status::set_status(
                pool,
                name,
                trovato_kernel::plugin::status::STATUS_DISABLED,
            )
            .await;
        }
    }
    for name in plugins {
        install_and_enable(pool, name)
            .await
            .unwrap_or_else(|e| panic!("install {name}: {e}"));
        trovato_kernel::plugin::status::mark_tap_install_called(pool, name)
            .await
            .unwrap();
    }
}

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub cookies: String,
    pub body: String,
}

impl App {
    pub async fn get(&self, path: &str, cookies: &str) -> Response {
        let mut req = axum::http::Request::get(path);
        if !cookies.is_empty() {
            req = req.header(axum::http::header::COOKIE, cookies);
        }
        self.send(req.body(axum::body::Body::empty()).unwrap())
            .await
    }

    pub async fn send(&self, request: axum::http::Request<axum::body::Body>) -> Response {
        use tower::ServiceExt;
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status().as_u16();
        let header = |name| {
            response
                .headers()
                .get_all(name)
                .iter()
                .filter_map(|v| v.to_str().ok())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let content_type = header(axum::http::header::CONTENT_TYPE).join(", ");
        let cookies = header(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|c| c.split(';').next().map(str::to_string))
            .collect::<Vec<_>>()
            .join("; ");
        let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap();
        Response {
            status,
            content_type,
            cookies,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }

    /// Create an ordinary member holding the named demo roles, and log in.
    ///
    /// Returns the session cookie and the new user's id, because a test that
    /// posts a form usually also has to read back the row the post wrote.
    ///
    /// **Not an administrator.** That is the whole point of it beside
    /// [`Self::login_as_new_admin`]: the administrator flag bypasses access
    /// checks and half the kernel's screens are gated on it, so a test that only
    /// ever logs in as an administrator cannot tell a working permission from a
    /// bypassed one. Every authenticated user additionally holds whatever the
    /// `authenticated user` role grants, which is where `view own profile` and
    /// `create content` come from; `demo/config` has to be imported for those to
    /// exist.
    ///
    /// Roles are named as `demo/config` names them: `"editor"`, `"publisher"`,
    /// `"viewer"`. An unknown name panics rather than silently producing a user
    /// with fewer permissions than the test believes it asked for.
    pub async fn login_as_new_member(&self, roles: &[&str]) -> (String, Uuid) {
        let id = Uuid::now_v7();
        let name = format!("ritrovo-test-member-{}", id.simple());
        let cookies = self.create_and_log_in(id, &name, false).await;

        for role in roles {
            let role_id: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
                .bind(role)
                .fetch_optional(self.state.db())
                .await
                .unwrap()
                .unwrap_or_else(|| {
                    panic!("no role named {role}; import demo/config before asking for it")
                });
            sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
                .bind(id)
                .bind(role_id)
                .execute(self.state.db())
                .await
                .unwrap();
        }

        // The role was granted after the session was established, and the
        // permission service caches per user. Log in again so the session's
        // context carries the roles this test just asked for.
        if roles.is_empty() {
            (cookies, id)
        } else {
            (self.log_in(&name).await, id)
        }
    }

    /// Create a site administrator and log in through `/user/login/json`,
    /// returning the session cookie.
    pub async fn login_as_new_admin(&self) -> String {
        let id = Uuid::now_v7();
        let name = format!("ritrovo-test-admin-{}", id.simple());
        self.create_and_log_in(id, &name, true).await
    }

    /// POST a form-urlencoded body, the way a browser submits a `<form>`.
    ///
    /// `application/x-www-form-urlencoded` and no `X-CSRF-Token` header, because
    /// that is what a plain HTML form sends and the no-JavaScript claim is only
    /// worth anything if the tests submit the way a browser does. A token
    /// travels in the body, under `_token` for a plugin-served route and `_csrf`
    /// for a kernel one.
    pub async fn post_form(&self, path: &str, body: &str, cookies: &str) -> Response {
        let mut request = axum::http::Request::post(path)
            .header("content-type", "application/x-www-form-urlencoded");
        if !cookies.is_empty() {
            request = request.header(axum::http::header::COOKIE, cookies);
        }
        self.send(
            request
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    /// Insert a user with a known password, then log in as them.
    async fn create_and_log_in(&self, id: Uuid, name: &str, is_admin: bool) -> String {
        use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};

        let hash = argon2::Argon2::default()
            .hash_password(TEST_PASSWORD.as_bytes(), &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();
        sqlx::query(
            "INSERT INTO users (id, name, pass, mail, status, is_admin) \
             VALUES ($1, $2, $3, $4, 1, $5)",
        )
        .bind(id)
        .bind(name)
        .bind(&hash)
        .bind(format!("{name}@example.test"))
        .bind(is_admin)
        .execute(self.state.db())
        .await
        .unwrap();

        self.log_in(name).await
    }

    /// Log in an existing test user, returning the session cookie.
    async fn log_in(&self, name: &str) -> String {
        clear_rate_limits().await;
        let response = self
            .send(
                axum::http::Request::post("/user/login/json")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "username": name, "password": TEST_PASSWORD })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(response.status, 200, "login failed: {}", response.body);
        assert!(!response.cookies.is_empty(), "login set no session cookie");
        response.cookies
    }
}

/// The password every user these suites create is given.
const TEST_PASSWORD: &str = "correct horse battery staple";

/// Forget every rate-limit counter in Redis.
///
/// **Not a workaround for the limiter; test isolation from it.** The kernel
/// limits logins per client IP, and it is right to: a handful of attempts a
/// minute from one address is what a person does and a lot more is what a
/// password guesser does. But every request in these suites arrives from
/// 127.0.0.1, and a suite that signs a fresh user in for each of its tests
/// spends that budget in seconds — so without this the first few tests pass and
/// the rest fail with 429, in an order that depends on how fast the machine is.
/// That is a flake, and it hides real failures behind a fake one.
///
/// It clears counters rather than raising limits, so the limiter that ships is
/// the limiter under test everywhere else, and a suite that wants to prove the
/// limit works can still spend it deliberately.
///
/// Redis failures are ignored: the kernel's own limiter fails open on a Redis
/// error, so a test run against a Redis that cannot be reached is one where
/// nothing was counted in the first place.
pub async fn clear_rate_limits() {
    let Ok(client) = redis::Client::open(redis_url()) else {
        return;
    };
    let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
        return;
    };
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("rate:*")
        .query_async(&mut conn)
        .await
        .unwrap_or_default();
    for key in keys {
        let _: Result<i64, _> = redis::cmd("DEL").arg(&key).query_async(&mut conn).await;
    }
}

/// Parse a tap or route body as JSON, naming the body when it is not.
pub fn json(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|e| panic!("not JSON ({e}): {raw}"))
}

/// Create a published `conference` Item through `ItemService::create`, which is
/// what fires `tap_item_presave` and `tap_item_insert`, as the author the kernel
/// migrations seed.
pub async fn create_conference(
    pool: &PgPool,
    items: &ItemService,
    title: &str,
    stage: Option<&str>,
    fields: serde_json::Value,
) -> trovato_kernel::models::Item {
    let author = any_user(pool).await;
    items
        .create(
            trovato_kernel::models::CreateItem {
                item_type: "conference".to_string(),
                title: title.to_string(),
                author_id: author,
                status: Some(1),
                promote: None,
                sticky: None,
                fields: Some(fields),
                stage_id: stage.map(|s| Uuid::parse_str(s).unwrap()),
                language: None,
                log: None,
            },
            &UserContext::authenticated(author, vec!["create conference content".to_string()]),
        )
        .await
        .unwrap_or_else(|e| panic!("create conference {title}: {e:#}"))
}

/// A signed-in visitor holding exactly these permissions.
///
/// Synthetic: the permission set is handed over rather than resolved, which is
/// right for testing a tap's own logic and wrong for testing whether anybody can
/// actually hold those permissions. For that, use [`visitor_named`].
pub async fn visitor(pool: &PgPool, permissions: &[&str]) -> UserContext {
    UserContext::authenticated(
        any_user(pool).await,
        permissions.iter().map(|p| (*p).to_string()).collect(),
    )
}

/// A real user, with the permissions their real roles really grant.
///
/// The difference from [`visitor`] is the whole point of it. `visitor` proves a
/// tap decides correctly when handed a permission set; this proves the set a
/// signed-in person actually carries, resolved the way the kernel resolves it
/// for a request: `PermissionService::load_user_permissions` over the
/// `user_roles` join, then `context_from_permissions`, which is also what adds
/// the administrator marker.
///
/// So a test using this fails if `demo/config/role.*.yml` stops granting what
/// the plugin reads — which is exactly the failure that a synthetic context
/// cannot see, and the one that would take the demo's editorial workflow down.
pub async fn visitor_named(pool: &PgPool, username: &str) -> UserContext {
    let user = trovato_kernel::models::User::find_by_name(pool, username)
        .await
        .unwrap_or_else(|e| panic!("look up {username}: {e}"))
        .unwrap_or_else(|| panic!("no user named {username}"));

    let permissions =
        trovato_kernel::permissions::PermissionService::new(pool.clone(), Duration::from_secs(60))
            .load_user_permissions(&user)
            .await
            .unwrap_or_else(|e| panic!("load permissions for {username}: {e}"));

    trovato_kernel::permissions::context_from_permissions(&user, permissions)
}
