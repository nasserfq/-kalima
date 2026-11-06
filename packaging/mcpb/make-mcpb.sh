#!/usr/bin/env bash
# Claude desktop extension (.mcpb) for كلمة on macOS: double-clicking it opens Claude's own
# «Install» window, which adds كلمة's MCP server to Claude without editing any file.
#
# The extension holds only its description and the كلمة logo: it runs the MCP server inside the
# installed app (/Applications/كلمة.app/Contents/MacOS/kalima-cli), so the server updates with the
# app and the file stays a few kilobytes. On other systems, and for other AI apps, كلمة's own
# Help › Connect to AI Apps window sets it up.
#
# usage: packaging/mcpb/make-mcpb.sh OUT_DIR VERSION
set -euo pipefail
OUT="${1:?out dir}"
VERSION="${2:?version}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp "$ROOT/assets/app-icon/wordcraft-macos-512.png" "$WORK/icon.png"
cat > "$WORK/manifest.json" <<JSON
{
  "manifest_version": "0.3",
  "name": "kalima",
  "display_name": "كلمة (Kalima)",
  "version": "$VERSION",
  "description": "اكتب ونسّق مستندات Word بكلامك في كلمة، معالج النصوص العربي.",
  "long_description": "كلمة معالج نصوص عربي مبني على WordCraft. بعد تثبيت هذه الإضافة يكتب Claude المستندات وينسّقها ويضيف الجداول والأنماط ويحفظها بصيغة Word أو PDF عندما تطلب منه ذلك. يحتاج كلمة مثبتًا في مجلد التطبيقات.\n\nKalima is an Arabic word processor based on WordCraft. With this extension Claude can write and format documents, add tables and styles, and save them as Word or PDF. Needs Kalima installed in Applications.",
  "author": { "name": "Nasser Alqahtani", "url": "https://github.com/nasserfq/kalima" },
  "homepage": "https://github.com/nasserfq/kalima",
  "repository": { "type": "git", "url": "https://github.com/nasserfq/kalima.git" },
  "license": "MIT OR Apache-2.0",
  "icon": "icon.png",
  "server": {
    "type": "binary",
    "entry_point": "/Applications/كلمة.app/Contents/MacOS/kalima-cli",
    "mcp_config": {
      "command": "/Applications/كلمة.app/Contents/MacOS/kalima-cli",
      "args": ["mcp"]
    }
  },
  "keywords": ["word", "docx", "arabic", "documents", "كلمة"],
  "compatibility": { "platforms": ["darwin"] }
}
JSON
python3 -m json.tool "$WORK/manifest.json" >/dev/null
mkdir -p "$OUT"
rm -f "$OUT/kalima-claude-extension-macos.mcpb"
(cd "$WORK" && zip -q -X "kalima-claude-extension-macos.mcpb" manifest.json icon.png)
mv "$WORK/kalima-claude-extension-macos.mcpb" "$OUT/"
echo "$OUT/kalima-claude-extension-macos.mcpb"
