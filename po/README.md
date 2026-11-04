# Translations

Voltkit's interface, command-line output, help and man pages are translated with
ordinary gettext catalogs:

| File | Language |
|---|---|
| `voltkit.pot` | the template: every message in English |
| `es.po` | Spanish (neutral Latin American, *tú*) |
| `de.po` | German (*Sie*, following GNOME's German conventions) |

The catalogs are compiled into the binary (see `src/i18n.rs`), so there are no
`.mo` files to build or install. Voltkit picks the language from `LANGUAGE`,
`LC_ALL`, `LC_MESSAGES` or `LANG`, like any gettext program; anything without a
catalog shows in English. Try a language with `LANGUAGE=de voltkit`.

## Fixing a translation

Edit the `msgstr` in the `.po` file with any text editor or a PO editor such as
Poedit or GNOME's Translation Editor, then `cargo build` and run Voltkit. Keep:

- every `{}` placeholder, the same number of times. Use `{0}`, `{1}`… to change
  their order: `{1} de {0}`. `{{` and `}}` are literal braces.
- command names, options and values in backticks or code, such as
  `voltkit daemon`, `--apply`, `on`, `off`: they're the same in every language.
- leading and trailing spaces and line breaks.

`cargo test` checks the placeholders. A message marked `#, fuzzy` or left empty
shows in English.

## After changing the code

New or changed text in the sources (anything in `tr()`, `trf!`, `trn()`,
`trnf!` or `N_!`) and command-line help must reach the catalogs:

```bash
python3 po/update.py          # refresh voltkit.pot and merge it into each .po
python3 po/update.py --check  # what CI runs: fails on stale or untranslated messages
cargo run -- manpages man     # the man pages, in every language
```

New messages arrive untranslated in each `.po`; translate them before merging.

## Adding a language

1. Copy `voltkit.pot` to `LANG.po` (for example `fr.po`) and set its
   `Language:` and `Plural-Forms:` header lines.
2. Add the language to `LANGUAGES` in `src/i18n.rs` and in `po/update.py`. If its
   plural rule isn't "singular for exactly one", extend `trn` in `src/i18n.rs`.
3. Translate, then run `python3 po/update.py` and `cargo run -- manpages man`,
   and add `man/LANG/` to the .deb assets in `Cargo.toml` and to
   `packaging/rpm/voltkit.spec`.
4. Add the language's name, summary and description to the desktop files and
   `data/io.github.arios4k.Voltkit.metainfo.xml`.
