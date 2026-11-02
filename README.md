# filejam

A fast, native, dual-pane SFTP client written in Rust with GPUI, in the
spirit of ForkLift. Its look and widget vocabulary come from dbdelve.

## Running

```sh
cargo run            # debug
cargo run --release  # optimised
```

## What it does

- Two panes side by side. Each one shows this computer or any saved site.
  The focused pane is the brighter one.
- Sidebar with Favorites (Home, Desktop, Documents, Downloads, Computer) and
  saved sites. Each site shows a connection dot and has Edit and Disconnect
  buttons when you hover it.
- SFTP over SSH, using `russh` and `russh-sftp` (pure Rust). It signs in
  with the SSH agent first, then a key file (or `~/.ssh/id_ed25519`,
  `id_ecdsa` and `id_rsa`), then a password (also sent as
  keyboard-interactive).
- Host keys are checked against `~/.ssh/known_hosts`. A new key asks you to
  trust its SHA256 fingerprint. A changed key is refused.
- Copy between the panes with F5, the toolbar button or drag and drop. That
  covers upload, download, local to local and remote to remote. Folders are
  copied recursively, with progress, cancel and preserved modification
  times. You are asked before anything is overwritten.
- New folder, rename, recursive delete (with confirmation), a filter,
  sorting by name, size, date or kind, hidden files, a breadcrumb (double
  click it to type a path, `~` works) and back/forward history.
- Tabs in each pane, each its own folder, selection and history. Tabs on one
  site share its connection, and closing the last of them ends it. Right click
  a tab or a site for what it offers.
- Themes: Filejam Black, Dark and Light, Gruvbox, Catppuccin, GitHub, Tokyo
  Night, Dracula and One, each dark and light, all but Black also as glass.

Sites are saved in `sites.toml` in the platform config directory
(`~/Library/Application Support/filejam` on macOS). Passwords and
passphrases go in the system keychain, never in the file.

## Keys

| Key | Action |
|---|---|
| ↑ ↓, ⇧↑ ⇧↓, ⌘A | move, extend the selection, select all |
| ↩ / ⌘↓ | open |
| ⌫ / ⌘↑ | enclosing folder |
| ⌘[ ⌘] | back / forward |
| Tab | switch pane |
| F5 / ⌘⇧C | copy to the other pane |
| F7 / ⌘⇧N | new folder |
| F2 | rename |
| F8 / ⌘⌫ | delete |
| ⌘R | refresh |
| ⌘L | go to folder |
| ⌘⇧. | show hidden files |
| ⌘N | new site |
| ⌘T, ⌘W | new tab, close tab |
| ⌃⇥, ⌃⇧⇥ | next tab, previous tab |
| ⌘⇧D | disconnect |
| ⌘⇧T | transfers |
| ⌘⌥S | sidebar |

## Tests

```sh
cargo test
# Against a real server:
docker run --rm -d --name sftp -p 2222:22 atmoz/sftp:alpine tester:secret:::upload
FILEJAM_TEST_SFTP=localhost:2222:tester:secret cargo test -- --ignored
```

`FILEJAM_KNOWN_HOSTS` points host-key checks at a different file. The live
test uses it so it never touches `~/.ssh/known_hosts`.

## Notes

- `vendor/` holds the gpui and gpui-component patches from dbdelve (backdrop
  blur, alpha compositing). The versions are pinned exactly, the same as in
  dbdelve.
- The licence is GPL-3.0, because code comes from dbdelve.
