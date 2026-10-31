# كلمة

**كلمة** معالج نصوص عربي بتصميم نجدي، مبني على [WordCraft](https://github.com/storytold/wordcraft).

- الواجهة باللغة العربية.
- ألوان مستوحاة من العمارة النجدية: لون الطين، وبياض الجص، والأحمر الطيني للأبواب المزخرفة.
- الواجهة من اليمين إلى اليسار: قيد العمل.

**Kalima (كلمة)** is an Arabic word processor with a Najdi-inspired design, based on
[WordCraft](https://github.com/storytold/wordcraft) by the ArtCraft team. It is an independent
project and is not made, sponsored or endorsed by the ArtCraft team.

## Build

Same as WordCraft: install Rust, then from a checkout of
[storytold/craft-i18n](https://github.com/storytold/craft-i18n) at the commit in `craft-i18n.rev`:

```sh
CRAFT_I18N_DIR="$HOME/path/to/craft-i18n" cargo run --release -p wordcraft   # must be an absolute path
```

Pick العربية in File › Options › Interface language (or run with an Arabic system language).

## License

MIT OR Apache-2.0, like WordCraft: see `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and
`ATTRIBUTION.md`.
