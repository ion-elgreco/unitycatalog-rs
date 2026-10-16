# Web UI

The web UI shows the catalogs of a server, and creates, edits and deletes
them and what they hold. It is a Leptos port of upstream's React UI, with
the same pages, and a page for your access tokens. `uc-server` serves it at
`/`, on the API's origin.

![The catalogs, and the tree of the "unity" catalog](assets/ui/catalogs.png)

## Start the UI

- `uc dev` runs a server on `http://localhost:8080`, with the UI.
- The [image](docker.md) and the [Helm chart](kubernetes.md) serve the UI
  at `/`. `ui.enabled = false` (`UC_UI_ENABLED=false`, or `ui.enabled:
  false` in the chart) turns it off.
- A server that you start yourself serves the UI that `pip install
  'unitycatalog-rs[ui]'` installed, or the folder of `ui.dir`
  (`UC_UI_DIR`).
- `uc web` serves the UI on `http://localhost:3000` and sends its API calls
  on to another server (`--server`). The calls carry the token of the `uc`
  commands. The API docs come from that server too.

When the server needs a sign-in, the UI shows its login page first. The
"Sign in" button goes to the server's identity provider, or the page tells
you to open the UI through the auth proxy.
[Sign-in and authorization](server/auth.md#sign-in-to-the-web-ui) tells how
to set up the sign-in.

![The login page](assets/ui/login.png)

## Find your way

- The header: "Catalogs" opens the list of catalogs. "API docs" opens the
  server's REST API in Swagger UI (`/docs`), in a new tab. The avatar,
  after a sign-in, shows your name and email, "Access tokens" and "Log
  out". If the server is set up for it, "Log out" also signs you out of the
  identity provider.
- "Browse", on the left: the tree of the catalogs, their schemas, and the
  tables, volumes, functions and models of each schema. A click on the
  arrow opens a node; a click on a name opens its page. The page that
  shows is selected in the tree. The arrow keys move between the names;
  the right arrow opens a node, and the left arrow closes it.
- Each page has a path at the top, such as "Catalogs / unity / default".
  Click a part of it to go back up.
- The three dots (⋮) at the right of a page's title open its actions, such
  as "Delete Catalog".
- Lists show 10 rows on each page. Above 50 rows, you can show 20, 50 or
  100 rows. "Search" shows the rows whose name contains the text.
- A page shows the data that the UI has at once, and loads it again each
  time it opens. When you come back to the window, the page loads again
  too. So a change that you make elsewhere, such as a token that you make
  with `uc`, shows without a reload of the page.

## Catalogs

1. Click **Catalogs** in the header to see the list of catalogs. Click a
   row to open the catalog.
2. To make a catalog, click **Create Catalog**. Give the name, and a
   comment if you want one. Then click **Create**: the catalog's page
   opens.

![Create catalog](assets/ui/catalog-create.png)

3. A catalog's page shows its description, its schemas and its details.
   Click the pencil to edit the description.

![A catalog](assets/ui/catalog.png)

4. To delete the catalog, click ⋮, then **Delete Catalog**, then
   **Delete**. The server deletes only a catalog without schemas. An
   admin can tick **Also delete its schemas, …**: the dialog then counts
   what goes with the catalog, and **Delete** asks again, in a second
   dialog, before **Delete everything**.

![Delete a catalog](assets/ui/catalog-delete.png)

## Schemas

1. To make a schema, open its catalog and click **Create Schema**.

![Create schema](assets/ui/schema-create.png)

2. A schema's page lists its tables. The buttons beside the search box
   show its volumes, functions and models. The tree's "Tables", "Volumes",
   "Functions" and "Models" open the same lists.

![A schema](assets/ui/schema.png)

3. To delete the schema, click ⋮, then **Delete Schema**. The server
   deletes only an empty schema; an admin can tick **Also delete its
   tables, …**, as for catalogs.

## Tables

A table's page shows its description, its columns with their types, and
its details: when it was made and changed, and its data source format.
To delete the table, click ⋮, then **Delete Table**. The UI does not make
tables: make them with `uc table create`, an engine or the API.

![A table](assets/ui/table.png)

## Volumes

A volume's page shows its description and its details: when it was made
and changed, and its type. Click the pencil to edit the description. To
delete the volume, click ⋮, then **Delete Volume**.

![A volume](assets/ui/volume.png)

![Edit a volume's description](assets/ui/volume-edit.png)

## Functions

A function's page shows its description, its definition and its details,
such as its language. To delete the function, click ⋮, then **Delete
Function**.

![A function](assets/ui/function.png)

## Models

1. Open a schema and click **Models** to see its registered models.
2. To make a model, click ⋮ on the schema's page, then **Create Registered
   Model**.

![The models of a schema, and the schema's actions](assets/ui/models.png)

![Create a registered model](assets/ui/model-create.png)

3. A model's page shows its description, its versions and its details.
   The icon of a version tells its status: ready (green), pending (gray)
   or failed (red); point to the icon to read it.
4. To delete the model, click ⋮, then **Delete Model**. Delete its versions
   first, or, as an admin, tick **Also delete its versions**, as for
   catalogs.

![A model and its versions](assets/ui/model.png)

## Model versions

Click a version on the model's page. The page shows the version's
description, its details (when and by whom it was made and changed, its
status and training run) and the model's details. To delete the version,
click ⋮, then **Delete Model Version**.

![A model version](assets/ui/model-version.png)

## Access tokens

An access token lets a script or a tool call the API as you, without a
sign-in. It sends the token in the header `Authorization: Bearer <token>`;
`uc` takes it with `--auth_token`.

1. Click the avatar, then **Access tokens**. The list shows your tokens:
   the name, the first characters, and when each one was made and expires.
   An expired token shows in red.
2. To make a token, click **Create Token**. Give a name, and choose when
   the token expires: in 7, 30 or 90 days, at the end of a date that you
   choose (**Custom**), or **Never**. Then click **Create**.

![Create access token](assets/ui/token-create.png)

3. The new token shows above the list. Click **Copy**, and keep the token in
   a safe place. The server keeps only a hash of the token, so the page
   cannot show it again after you leave the page.

![A new access token](assets/ui/tokens.png)

4. To stop a token, click **Delete** in its row. Scripts and tools that use
   the token can then no longer call the API.

Tokens belong to a signed-in user: without a sign-in, the page has no
tokens. The [terminal UI](cli/index.md#the-terminal-ui) has the same on its
Tokens tab.

## Upstream's React UI

The server keeps upstream's REST API, so upstream's UI works against it
too. Its development server sends the API calls to `http://localhost:8080`
(`proxy` in its `package.json`), where `uc-server` listens by default:

```bash
git clone https://github.com/unitycatalog/unitycatalog.git
cd unitycatalog/ui
bun install
bun run start
```

Open `http://localhost:3000`. A UI on another origin than the API needs
CORS: the server lets every origin call the API, unless
`server.cors_allow_origins` (`UC_SERVER_CORS_ALLOW_ORIGINS`) names the
allowed ones.
