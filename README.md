<p align="center">
  <img src="assets/app-icon/hicolor/128x128/apps/ai.storyteller.wordcraft.png" width="128" height="128" alt="كلمة">
</p>

<h1 align="center">كلمة</h1>

<p align="center">
  <b>معالج نصوص عربي، مجاني ومفتوح المصدر</b><br>
  An Arabic word processor, free and open source
</p>

<p align="center">
  <a href="https://github.com/nasserfq/kalima/releases/latest"><b>⬇︎ تنزيل · Download</b></a>
  &nbsp;·&nbsp; macOS &nbsp;·&nbsp; Windows &nbsp;·&nbsp; Linux
</p>

---

<div dir="rtl">

## عن كلمة

كلمة معالج نصوص مصمم للعربية أولًا: الواجهة عربية ومن اليمين إلى اليسار، والمستندات الجديدة تُكتب بالعربية وتُدقق إملائيًا.

- **واجهة عربية بالكامل** من اليمين إلى اليسار، مع خيار الإنجليزية.
- **مستندات من اليمين إلى اليسار** افتراضيًا.
- **تدقيق إملائي عربي** بمدقق النظام في macOS وWindows.
- **ملفات Word** ‏(‎.docx‎) فتحًا وحفظًا.

## التثبيت

| النظام | الملف | الخطوات |
|---|---|---|
| macOS ‏(Apple Silicon وIntel، الإصدار 11 فأحدث) | `kalima-…-macos-universal.dmg` | افتح الملف واسحب كلمة إلى «التطبيقات». موقّع وموثَّق من Apple. |
| Windows | `kalima-…-windows-x64-portable.zip` | فك الضغط وشغّل `Kalima.exe`. |
| Linux | `kalima-…-linux-x86_64.tar.gz` | فك الضغط وشغّل `kalima`. |

التحديث تلقائي: عند صدور إصدار جديد يعرض كلمة «تحديث وإعادة تشغيل»، وتبقى إعداداتك ومستنداتك كما هي.

## التحكم بكلمة من الذكاء الاصطناعي (MCP)

كلمة جاهز للذكاء الاصطناعي: يأتي مع خادم [MCP](https://modelcontextprotocol.io)، وهو معيار مفتوح تستخدمه تطبيقات الذكاء الاصطناعي لتشغيل البرامج. بعد ربطه تطلب بكلامك، مثلًا «اكتب خطابًا من صفحتين، واجعل العنوان عريضًا، وأضف جدولًا بالتكاليف، واحفظه PDF»، فينفّذه الذكاء الاصطناعي في كلمة خطوة بخطوة: الكتابة والتنسيق والجداول والأنماط وتتبع التغييرات وقراءة المستند والحفظ بصيغة Word أو PDF.

يعمل مع أي تطبيق يدعم MCP، مثل Claude ‏(التطبيق وClaude Code)، وCodex من OpenAI، وCursor، وVS Code مع Copilot، وGemini CLI، وLM Studio ‏(نماذج تعمل على جهازك دون إنترنت). الخادم مجاني ويعمل على جهازك، أما الذكاء الاصطناعي فمن التطبيق الذي تختاره وبحسب اشتراكك فيه.

**الطريقة الأسهل، بنقرة:** في كلمة افتح تبويب «مساعدة» ← «ربط مع الذكاء الاصطناعي» (أو من قائمة «ملف»). تظهر تطبيقات الذكاء الاصطناعي المثبتة على جهازك، مثل Claude وClaude Code وCodex وCursor وVS Code وWindsurf وGemini CLI وLM Studio، فاضغط «ربط» بجانب التطبيق ثم أعد تشغيله. ولأي تطبيق آخر يدعم MCP اضغط «نسخ الإعداد» والصقه في إعداداته.

اذكر **kalima** في طلبك، مثل «استخدم أدوات kalima لإنشاء مستند جديد واكتب فيه…»، لأن «كلمة» تعني أيضًا Word، فقد يفتح بعض التطبيقات (مثل Codex) برنامج Microsoft Word بدلًا منه.

على Mac يمكنك أيضًا تنزيل ملف `kalima-claude-extension-macos.mcpb` من [صفحة الإصدارات](https://github.com/nasserfq/kalima/releases/latest) وفتحه، فيعرض تطبيق Claude نافذة «تثبيت» ويضيف كلمة بنقرة.

**الإعداد اليدوي (إن أردت):**

**1. مكان الخادم:** يُثبَّت مع كلمة ويُحدَّث معه.

| النظام | المسار |
|---|---|
| macOS | `/Applications/كلمة.app/Contents/MacOS/kalima-cli` |
| Windows | `kalima-cli.exe` في مجلد كلمة، بجانب `Kalima.exe` |
| Linux | `kalima-cli` في مجلد كلمة، بجانب `kalima` |

**2. أضفه إلى تطبيق الذكاء الاصطناعي:**

- **تطبيق Claude:** الإعدادات ← «Developer» ← «Edit Config»، ثم أضف ما يلي إلى الملف `claude_desktop_config.json` وأعد تشغيل Claude:

<div dir="ltr">

```json
{
  "mcpServers": {
    "kalima": {
      "command": "/Applications/كلمة.app/Contents/MacOS/kalima-cli",
      "args": ["mcp"]
    }
  }
}
```

</div>

- في Windows ضع مسار `kalima-cli.exe` بدل المسار السابق، مع مضاعفة الشرطة المائلة، مثل `"C:\\Users\\اسمك\\Kalima\\kalima-cli.exe"`.
- **Cursor وGemini CLI وLM Studio** وأغلب التطبيقات الأخرى: الإعداد نفسه (`mcpServers`) في ملف MCP الخاص بكل تطبيق (`~/.cursor/mcp.json`، ‏`~/.gemini/settings.json`، ‏`mcp.json` في LM Studio).
- **Claude Code:** <code dir="ltr">claude mcp add kalima -- "/Applications/كلمة.app/Contents/MacOS/kalima-cli" mcp</code>
- **Codex:** <code dir="ltr">codex mcp add kalima -- "/Applications/كلمة.app/Contents/MacOS/kalima-cli" mcp</code>

بهذا يعمل الذكاء الاصطناعي على المستندات في الخلفية دون نافذة، ثم يحفظها حيث تطلب.

**التحكم بنافذة كلمة المفتوحة (اختياري):** لتشاهد الذكاء الاصطناعي يكتب في النافذة نفسها، شغّل كلمة مع منفذ تحكم، واجعل `args` هي <code dir="ltr">["mcp", "--connect", "127.0.0.1:7981"]</code>:

<div dir="ltr">

```sh
"/Applications/كلمة.app/Contents/MacOS/WordCraft" --control 7981   # macOS
Kalima.exe --control 7981                                           # Windows
./kalima --control 7981                                             # Linux
```

</div>

منفذ التحكم محلي فقط ومحمي بمفتاح سري يكتبه كلمة في مجلد إعداداتك، فلا يصل إليه أحد من الشبكة.

## ما أضافه ناصر القحطاني إلى WordCraft

- الواجهة العربية افتراضيًا، والاتجاه من اليمين إلى اليسار في الواجهة والمستندات.
- التدقيق الإملائي العربي بمدقق النظام في macOS وWindows.
- خط Noto Sans Arabic بحجم مريح للقراءة.
- خادم MCP ضمن التنزيلات لتتحكم تطبيقات الذكاء الاصطناعي بكلمة، ونافذة «ربط مع الذكاء الاصطناعي» تربطه بتطبيقات الذكاء الاصطناعي بنقرة، وإضافة Claude ‏(`.mcpb`).
- الاسم والشعار، وتطبيق Mac موقّع وموثَّق لأجهزة Apple Silicon وIntel، وتنزيلات Windows وLinux.

## شكر

كلمة مبني على [WordCraft](https://github.com/storytold/wordcraft)، مع الشكر لفريق ArtCraft على عملهم الأصلي. كلمة مشروع مستقل، وليس من إنتاج فريق ArtCraft ولا برعايته.

</div>

---

## About

Kalima (كلمة) is an Arabic-first word processor by Nasser Alqahtani: an Arabic, right-to-left
interface (English available), right-to-left Arabic documents by default, Arabic spell checking
(the macOS and Windows system spell checkers) and Word `.docx` files.

Nasser's additions to WordCraft: the Arabic interface as the default, right-to-left interface and
documents, Arabic spell checking through the macOS and Windows system spell checkers, the Noto Sans
Arabic font, the name and logo, and the signed universal Mac app and Windows and Linux downloads.

It is based on [WordCraft](https://github.com/storytold/wordcraft) by the ArtCraft team. Kalima is an
independent project, not made, sponsored or endorsed by the ArtCraft team.

## Use it from AI apps (MCP)

Kalima ships an [MCP](https://modelcontextprotocol.io) server, so Claude and any other MCP app
(Codex, Cursor, VS Code, Gemini CLI, LM Studio…) can write, format, read and save documents. It is
installed and updated with the app: `/Applications/كلمة.app/Contents/MacOS/kalima-cli` on macOS,
`kalima-cli.exe` beside `Kalima.exe` on Windows, `kalima-cli` beside `kalima` on Linux. Add it to
your client as the command `kalima-cli` with the argument `mcp` (Help ▸ Connect to AI Apps does this
with one click for Claude, Claude Code, Codex, Cursor, VS Code, Windsurf, Gemini CLI and LM Studio; on
macOS the `kalima-claude-extension-macos.mcpb` release download installs it into Claude; by hand, for
example
`claude mcp add kalima -- "/Applications/كلمة.app/Contents/MacOS/kalima-cli" mcp`). To drive the
open window instead, start Kalima with `--control 7981` and use `mcp --connect 127.0.0.1:7981`.
Details: [docs/mcp.md](docs/mcp.md).

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
