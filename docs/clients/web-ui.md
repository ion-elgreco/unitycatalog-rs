# Web UI

The web UI shows the catalogs of a server, and creates, edits and deletes what they hold. It is a
port of upstream's UI, with the same pages, and a page for your access tokens. `uc-server` serves
it at `/`.

<video src="../assets/ui/web-ui.mp4" poster="../../assets/ui/catalogs.png" autoplay loop muted playsinline controls aria-label="Create a catalog, browse to a table, and delete the catalog"></video>

## Open it

- `uc dev` runs a server with the UI, and opens it in your browser.
- The [image](../server/docker.md) and the [Helm chart](../server/kubernetes.md) serve the UI at
  `/`.
- `uc web` serves the UI on <http://localhost:3000>, for another server (`--server`), with the
  token of `uc`.

When the server needs a sign-in, the UI shows its login page first: see
[Sign-in and authorization](../server/auth.md#sign-in-to-the-web-ui).

## Use it

- **Browse**, on the left, is the tree of catalogs, schemas, and the tables, volumes, functions
  and models of each schema. Click a name to open its page.
- **Create**: the button of a list, such as **Create Catalog** on the list of catalogs, or
  **Create Schema** on a catalog.
- **Edit** a description: the pencil beside it.
- **Delete**: ⋮ at the right of the page's title. The server deletes only a catalog, schema or
  model that holds nothing; an admin can tick a box to delete what it holds too.

The UI does not make tables: use `uc table create`, an engine or the API.

![A table](../assets/ui/table.png)

## Access tokens

An access token lets a script call the API as you. Click the avatar, then **Access tokens**:

1. Click **Create Token**, give a name, and choose when it expires.
2. Click **Copy**, and keep the token safe: the page shows it only once.
3. To stop a token, click **Delete** in its row.

![A new access token](../assets/ui/tokens.png)
