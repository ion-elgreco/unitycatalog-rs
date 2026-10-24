# Terminal UI

`uc` with no command, or `uc tui`, opens a terminal UI. It has a tab for each kind of object, the
list on the left, and the selected object, as JSON, on the right. It calls the server of the `uc`
commands, with their [sign-in](cli.md#sign-in).

```bash
uc
```

<video src="../assets/tui.mp4" poster="../../assets/tui.png" autoplay loop muted playsinline controls aria-label="Create a catalog, browse to a table, and delete the catalog"></video>

| Key | Action |
| --- | --- |
| ++tab++, ++shift+tab++ | Next or previous tab. |
| ++down++, ++up++ | Select an item. |
| ++enter++, ++backspace++ | Open the item, such as the schemas of a catalog; go back up. |
| ++page-down++, ++page-up++ | Scroll the details. |
| ++c++ | Create an item. |
| ++d++ | Delete the item, after a question. |
| ++r++ | Load the list again. |
| ++s++ | Change the server. |
| ++question++ | Show all keys. |
| ++q++ | Quit. |

On the tokens tab, ++c++ makes a personal access token. It shows once.

When you quit, the terminal UI keeps the server, the tab and the item that you opened, for the
next start.
