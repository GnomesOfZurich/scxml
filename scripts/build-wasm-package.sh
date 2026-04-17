#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="pkg"
TARGET="web"
FEATURES="wasm"
SCOPE=""
PACKAGE_NAME=""
SYNC_DEMO=0

usage() {
  cat <<'EOF'
Usage: ./scripts/build-wasm-package.sh [options]

Options:
  --scope <scope>              npm scope to pass to wasm-pack (for example: gnomes)
  --out-dir <dir>              output directory for wasm-pack (default: pkg)
  --target <target>            wasm-pack target (default: web)
  --features <features>        cargo features to enable (default: wasm)
  --with-xstate                shorthand for --features wasm,xstate
  --package-name <name>        override the generated npm package name
  --sync-demo                  replace demo/pkg with the freshly generated package
  --help                       show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scope)
      SCOPE="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --target)
      TARGET="$2"
      shift 2
      ;;
    --features)
      FEATURES="$2"
      shift 2
      ;;
    --with-xstate)
      FEATURES="wasm,xstate"
      shift
      ;;
    --package-name)
      PACKAGE_NAME="$2"
      shift 2
      ;;
    --sync-demo)
      SYNC_DEMO=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

cd "$ROOT_DIR"

build_cmd=(wasm-pack build --target "$TARGET" --out-dir "$OUT_DIR")
if [[ -n "$SCOPE" ]]; then
  build_cmd+=(--scope "$SCOPE")
fi
build_cmd+=(-- --features "$FEATURES")

"${build_cmd[@]}"

python3 - "$ROOT_DIR" "$OUT_DIR" "$PACKAGE_NAME" "$FEATURES" <<'PY'
import json
import pathlib
import sys

root_dir = pathlib.Path(sys.argv[1])
out_dir = root_dir / sys.argv[2]
package_name = sys.argv[3].strip()
features = {feature for feature in sys.argv[4].split(",") if feature}

package_json_path = out_dir / "package.json"
package_json = json.loads(package_json_path.read_text())

has_xstate = "xstate" in features

if package_name:
    package_json["name"] = package_name

if has_xstate:
    package_json["description"] = (
        "Browser-first WebAssembly package for parsing, validating, exporting, diffing, "
        "simulating, and converting between W3C SCXML and XState statecharts."
    )
else:
    package_json["description"] = (
        "Browser-first WebAssembly package for parsing, validating, exporting, diffing, "
        "and simulating W3C SCXML statecharts."
    )

package_json["bugs"] = {"url": "https://github.com/GnomesOfZurich/scxml/issues"}
package_json["homepage"] = "https://github.com/GnomesOfZurich/scxml#readme"
package_json["main"] = "scxml.js"
package_json["module"] = "scxml.js"
package_json["types"] = "scxml.d.ts"
package_json["exports"] = {
    ".": {
        "types": "./scxml.d.ts",
        "default": "./scxml.js",
    },
    "./package.json": "./package.json",
}
package_json["publishConfig"] = {"access": "public"}
package_json["sideEffects"] = False

files = ["README.md", "scxml_bg.wasm", "scxml.js", "scxml.d.ts"]
if (out_dir / "scxml_bg.wasm.d.ts").exists():
    files.append("scxml_bg.wasm.d.ts")
for license_name in ("LICENSE-APACHE", "LICENSE-MIT"):
    if (out_dir / license_name).exists():
        files.append(license_name)
package_json["files"] = files

keywords = [
    "scxml",
    "statechart",
    "state-machine",
    "workflow",
    "webassembly",
    "wasm",
    "xml",
    "visualization",
    "simulation",
]
if has_xstate:
    keywords.append("xstate")
package_json["keywords"] = keywords

package_json_path.write_text(json.dumps(package_json, indent=2) + "\n")
(out_dir / "README.md").write_text((root_dir / "README.md").read_text())
PY

if [[ "$SYNC_DEMO" -eq 1 ]]; then
  rm -rf "$ROOT_DIR/demo/pkg"
  mkdir -p "$ROOT_DIR/demo/pkg"
  cp -R "$ROOT_DIR/$OUT_DIR/." "$ROOT_DIR/demo/pkg/"

  rm -rf "$ROOT_DIR/demo/examples"
  mkdir -p "$ROOT_DIR/demo/examples"
  cp -R "$ROOT_DIR/examples/." "$ROOT_DIR/demo/examples/"
fi