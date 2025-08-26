# Ephemeral CLI

This is the official command-line interface for **Ephemeral Hubs**, a temporary, no-login-required space for text, files, and collaborative brainstorming.

This tool allows you to interact with ephemeral hubs directly from your terminal, making it easy to script and automate your workflows. It connects to the production backend at `https://ephemeral-hub.com` by default.

---

## Installation

```bash
cargo install ephemeral_hub
```

## Usage -- Manage a hub (create | pipe | upload | get)

```bash
# create -- Instantly generates a new ephemeral hub and returns its details.
ephemeral create
```

```bash
# pipe - Reads text from standard input and sends it to a hub's text bin.

cat log.txt | ephemeral pipe <API_URL>
```

```bash
# upload -- Uploads a local file to a hub.

ephemeral upload ./archive.zip <API_URL>
```

```bash
# get -- Downloads all content from a hub (text bin and all files) as a single .zip archive.

ephemeral get <API_URL>
```

```bash
Configuration
# By default, the CLI connects to the live server at https://ephemeral-hub.com. To override this for local development or to point to a self-hosted instance, you can set the EPHEMERAL_API_URL environment variable:

export EPHEMERAL_API_URL="http://127.0.0.1:3000"

ephemeral create
```


```txt
                .==-.                   .-==.
                 \()8`-._  `.   .'  _.-'8()/
                 (88"   ::.  \./  .::   "88)
                  \_.'`-::::.(#).::::-'`._/
                    `._... .q(_)p. ..._.'
                      ""-..-'|=|`-..-""
                      .""' .'|=|`. `"".
                    ,':8(o)./|=|\.(o)8:`.
                   (O :8 ::/ \_/ \:: 8: O)
                    \O `::/       \::' O/
                     ""--'         `--""


```