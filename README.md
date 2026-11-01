<p align="center">
  <img src="assets/app-icon/hicolor/128x128/apps/ai.storyteller.wordcraft.png" width="128" height="128" alt="كلمة">
</p>

<h1 align="center">كلمة</h1>

<p align="center">
  <b>معالج نصوص عربي، مجاني ومفتوح المصدر</b><br>
  An Arabic word processor, free and open source
</p>

<p align="center">
  <a href="https://github.com/nasserfq/-kalima/releases/latest"><b>⬇︎ تنزيل · Download</b></a>
  &nbsp;·&nbsp; macOS &nbsp;·&nbsp; Windows &nbsp;·&nbsp; Linux
</p>

---

<div dir="rtl">

## عن كلمة

كلمة معالج نصوص مصمم للعربية أولًا: الواجهة عربية ومن اليمين إلى اليسار، والمستندات الجديدة تُكتب بالعربية وتُدقق إملائيًا.

- **واجهة عربية بالكامل** من اليمين إلى اليسار، مع خيار الإنجليزية.
- **مستندات من اليمين إلى اليسار** افتراضيًا.
- **تدقيق إملائي عربي** بمدقق النظام في macOS.
- **ملفات Word** ‏(‎.docx‎) فتحًا وحفظًا.

## التثبيت

| النظام | الملف | الخطوات |
|---|---|---|
| macOS ‏(Apple Silicon وIntel، الإصدار 11 فأحدث) | `Kalima-macos.dmg` | افتح الملف واسحب كلمة إلى «التطبيقات». موقّع وموثَّق من Apple. |
| Windows | `Kalima-windows-x64.zip` | فك الضغط وشغّل `Kalima.exe`. |
| Linux | `Kalima-linux-x64.tar.gz` | فك الضغط وشغّل `kalima`. |

للتحديث: نزّل الإصدار الجديد واستبدل به القديم، وتبقى إعداداتك ومستنداتك كما هي.

## شكر

كلمة مبني على [WordCraft](https://github.com/storytold/wordcraft)، مع الشكر لفريق ArtCraft على عملهم الأصلي. كلمة مشروع مستقل، وليس من إنتاج فريق ArtCraft ولا برعايته.

</div>

---

## About

Kalima (كلمة) is an Arabic-first word processor by Nasser Alqahtani: an Arabic, right-to-left
interface (English available), right-to-left Arabic documents by default, Arabic spell checking
(macOS system speller) and Word `.docx` files.

It is based on [WordCraft](https://github.com/storytold/wordcraft) by the ArtCraft team. Kalima is an
independent project, not made, sponsored or endorsed by the ArtCraft team.

## Build from source

Install [Rust](https://rustup.rs), then build with the interface translations (the Arabic catalog is on
the `ar-wordcraft-missing-rows` branch of [nasserfq/craft-i18n](https://github.com/nasserfq/craft-i18n)):

```sh
git clone -b ar-wordcraft-missing-rows https://github.com/nasserfq/craft-i18n ../craft-i18n
CRAFT_I18N_DIR="$(cd ../craft-i18n && pwd)" cargo run --release -p wordcraft
```

`CRAFT_I18N_DIR` must be an absolute path, or the interface is English only.

Releases: pushing to the `release` branch builds, signs and publishes the downloads
(`.github/workflows/release.yml`).

## License

MIT OR Apache-2.0, like WordCraft: see [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE),
[NOTICE](NOTICE) and [ATTRIBUTION.md](ATTRIBUTION.md).
