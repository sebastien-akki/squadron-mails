# Mails

The mails your local site sends, caught by the project's Mailpit, next to your agents.

## Use

- **Mails** in an agent's toolbar opens the panel: the newest mails caught for the agent's project, a search box with Mailpit's syntax (`to:client@example.test subject:order`, `is:unread`), and each mail with its text, its links and its HTML shown in a sandboxed frame. **Delete all** empties Mailpit after a second click.
- Agents call `mails_list` and `mails_read` to check, while they test a checkout or a password reset, that the mail went out and to follow its link. Claude Code asks you first.
- Mailpit's address: the one set in the extension's settings, else the Mailpit or MailHog service of the project's Lando app (`lando info`, remembered and looked up again when it stops answering), else `http://localhost:8025`. The Lando app must be running.

## What it asks Squadron for

- **Run `curl`**: to read Mailpit's API, a local address the extension's own network access cannot reach.
- **Run `lando`**: `lando info` in the project's folder, to find its Mailpit.

## Build

```sh
rustup target add wasm32-wasip2
cargo test
cargo build --release --target wasm32-wasip2
```

Then in Squadron: Settings › Extensions › Add an extension › A folder on this computer.
