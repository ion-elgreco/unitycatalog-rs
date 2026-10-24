# uc-ui

Leptos rebuild of the upstream Unity Catalog OSS web UI (originally
React/TypeScript + Ant Design). Same routes, same pages, same data shapes —
all served as a client-side-rendered WASM app. One page is new: the
signed-in user's access tokens.
[docs/clients/web-ui.md](../../docs/clients/web-ui.md) shows how to use it.

## Build and run

The recipes of the justfile at the repository root:

```bash
just wasm       # release: crates/uc-ui/dist
just wasm-dev   # dev profile: faster, with panic messages and symbols
```

Each copies `public/` (`index.html`, `styles.css`, favicon, manifest,
`robots.txt`, logos) to `dist/`, and runs `cargo build -p uc-ui --target
wasm32-unknown-unknown`, then `wasm-bindgen --target web`, which writes
the wasm and its loader to `dist/assets/`. `just wasm` also writes
`uc_ui_bg.wasm.br` (brotli at quality 11) and `uc_ui_bg.wasm.gz` next to
the wasm, which the servers send as they are. The recipes need the
`wasm32-unknown-unknown` target, which rustup adds from
`rust-toolchain.toml`, and the wasm-bindgen CLI of the version in
`Cargo.lock`; `just wasm` also needs the `brotli` tool:

```bash
cargo install --locked wasm-bindgen-cli --version 0.2.121
brew install brotli   # or: apt install brotli
```

Then `uc-server` serves the UI from the root, on the API's origin:

```bash
UC_UI_DIR=crates/uc-ui/dist cargo run -p uc-server   # → http://localhost:8080
```

Or try it with dummy data, from the Python packages: `just demo` builds
them (`just develop`, into `.venv`), then runs `uc dev` from `.venv` on
http://localhost:8080, with its data in `.uc-demo`. The first run puts in
the data of `examples/seed_demo.py`; delete `.uc-demo` for new data.
`just demo-auth` does the same with sign-in, in `.uc-demo-auth`: the login
page goes to FerrisKey, an OpenID Connect provider in Docker on
http://localhost:8090, and users sign in as alice or bob (their names are
their passwords). The data is in RustFS, an S3-compatible store in Docker
on http://localhost:9000.

Or keep a server running and serve the UI with `uc web`, which sends its API
calls on to the server (`--server`, default http://localhost:8080):

```bash
UC_UI_DIR=crates/uc-ui/dist cargo run -p uc-cli -- web   # → http://localhost:3000
```

Both read the files on each request: after a change, run `just wasm-dev`
and load the page again.

## Layout

| Path                                                       | Page                 |
| ---------------------------------------------------------- | -------------------- |
| `/`                                                        | Catalogs list        |
| `/data/:catalog`                                           | Catalog details      |
| `/data/:catalog/:schema`                                   | Schema details       |
| `/data/:catalog/:schema/:table`                            | Table details        |
| `/volumes/:catalog/:schema/:volume`                        | Volume details       |
| `/functions/:catalog/:schema/:function`                    | Function details     |
| `/models/:catalog/:schema/:model`                          | Model details        |
| `/models/:catalog/:schema/:model/versions/:version`        | Model version detail |
| `/settings/tokens`                                         | Access tokens        |

## Stack

| Layer      | Choice                | Upstream equivalent          |
| ---------- | --------------------- | ---------------------------- |
| Framework  | `leptos` 0.7 (CSR)    | React 18                     |
| Router     | `leptos_router` 0.7   | React Router 6               |
| HTTP       | `uc-client` (this repo) | Axios                      |
| Types      | `uc-client`'s models, from `proto/` | OpenAPI-generated TS |
| UI library | CSS with antd's values | Ant Design 5                |
| Bundler    | wasm-bindgen (`just wasm`) | Create React App (Webpack) |

## Why own CSS

Ant Design has no Leptos port, and a port would be a large project. The
components of `src/components` and `src/layout` build the parts of antd
that upstream's UI uses (tree, table, pagination, modal, menu,
notification, tooltip). `public/styles.css` gives them antd 5's default
theme values, read from upstream's UI in a browser, and
`src/components/icons.rs` has antd's icon paths (`@ant-design/icons`,
MIT).

## Sign-in

The server signs users in with one of two modes
(`auth.mode`, or the variable `UC_AUTH_MODE`); the UI holds no identity
provider settings. Each variable below is also a key of `[auth]` in
`uc.toml`: `UC_AUTH_OIDC_ISSUER` is `oidc_issuer`. Upstream's
UI signs in in the browser instead, with a button per provider and
build-time `REACT_APP_*` settings.

`GET /auth/mode` tells the UI how the browser signs in: `{"mode": "oidc",
"logout_url": "/auth/logout"}`. The mode is the server's `auth.mode`;
`logout_url` is where "Log out" goes, or `null` to hide it. When the server
needs a sign-in, `/scim2/Me` answers 401 and the UI
shows its login page.

`oidc`: the login page has one "Sign in" button. It goes to the server's
`/auth/login`, which sends the browser to the OpenID Connect provider and
back to `/auth/callback`; the server then sets the `UC_TOKEN` cookie and
returns to the page. When the sign-in fails, the server logs why and sends
the browser to `/login?error=<code>`; the UI shows upstream's "Login
failed. Please contact your system administrator.", and a line for the
code: `provider`, `expired`, `token`, `user` (no active user has the
email) or `server`.

| Variable | |
| --- | --- |
| `UC_AUTH_PUBLIC_URL` | the server's external URL; the provider's client must allow `<public URL>/auth/callback`. With an `http://` URL, the cookies are not `Secure`, as browsers drop those on plain http |
| `UC_AUTH_OIDC_ISSUER` | the provider's issuer; its discovery document has the endpoints |
| `UC_AUTH_OIDC_CLIENT_ID`, `UC_AUTH_OIDC_CLIENT_SECRET` | the OAuth client; without a secret, a public client |
| `UC_AUTH_OIDC_SCOPES` | default `openid email profile` |

`forward`: an auth proxy in front of the server, such as oauth2-proxy or
Authelia, signs users in and names the user in its headers on every
request. A request without them gets the login page, which says to open the
UI through the proxy. The server takes them only from the proxy's addresses; other
requests, such as API clients that reach the server directly, sign in with
their tokens. The proxy must replace these headers on the requests it
passes on.

| Variable | |
| --- | --- |
| `UC_AUTH_FORWARD_TRUSTED_PROXIES` | the proxies' CIDRs or addresses, comma-separated: the address of the connection, never `X-Forwarded-For` |
| `UC_AUTH_FORWARD_EMAIL_HEADER`, `UC_AUTH_FORWARD_USER_HEADER` | the user: the email header (default `Remote-Email`), else the user header (default `Remote-User`) |
| `UC_AUTH_FORWARD_LOGOUT_URL` | the proxy's sign-out, where "Log out" goes; without it, the UI shows no "Log out", as the proxy would sign the user in again |

In both modes, a user signs in when their email names an active user of
the server (SCIM, `uc user create`), and "Log out" goes to `/auth/logout`.
Without a mode (`none`), the login page says that the server has no
sign-in for the web UI.
For a metastore admin, `/scim2/Me` names the role `admin` (with
authorization off, for everyone): the delete dialogs of catalogs, schemas
and models then offer to delete what they hold too (`force`), in two steps:
the dialog counts what goes, then a second dialog asks again. When the
server knows the user but does not let them in, such as a user
deactivated after sign-in, `/scim2/Me` answers 403 and the UI shows "No
access" with the server's message and "Log out". The UI asks `/scim2/Me`
again when the window gets the focus, as upstream does, and when a call
fails with 401 or 403, so that an ended session shows the login page.
Helm sets the variables from `auth.oidc` and `auth.publicUrl`, or from
`auth.forward`. `uc web` signs in as the other `uc` commands do: with the
token of `--auth_token`, else of `UC_CLI_TOKEN`, else of `token` in the `[cli]` table of
`uc.toml`. It answers `/auth/mode` with `{"mode": "token", "logout_url":
null}`: without a valid token, the login page says how to set one.

## How uc-server and `uc` find the bundle

`uc-server` looks for the bundle on startup: in `ui.dir` (`UC_UI_DIR`), else in
`share/unitycatalog-rs/ui` next to its `bin/` folder, where the Python
package `unitycatalog-rs-ui` (`python/ui`), the release tarballs and the
image install it. The bundle is mounted as a fallback service — API routes
still win first, anything else falls through to the static dir with SPA
fallback (`leptos_router` deep links work on hard refresh). `ui.enabled =
false` (`UC_UI_ENABLED`), or no bundle, means no UI.

`uc dev` runs uc-server with the same bundle, and `uc web --server <url>`
serves it on its own (localhost:3000), forwarding the UI's API calls
(`/api/...`) to that server, so that the UI and the API share one origin.

Both serve the files the same way (`uc_common::ui::files`):

- the wasm as the build compressed it, once:
  `uc_ui_bg.wasm.br` (brotli at quality 11) to browsers that accept
  brotli, `uc_ui_bg.wasm.gz` (gzip 9) to those that accept gzip, else the
  wasm (the release wasm: 6.6 MB; 551 KB with brotli, 934 KB with gzip).
  The servers never compress the wasm on a request: quality 11 takes
  seconds, and a dev build, which has no such files, sends its wasm as it
  is;
- the other files compressed on each request, with brotli or gzip at
  quality 5: they are small;
- every file with `Cache-Control: no-cache`: browsers check their kept copy
  with the server, which answers 304 when the file has not changed, so a
  new build takes effect at once;
- a missing file of `/assets/…` as a 404, never `index.html`.
