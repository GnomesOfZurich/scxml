#!/usr/bin/env bash
#
# Generate a shields.io-style SVG badge as a static file.
# No network calls — useful for repos that want self-contained badges.
#
# Usage: ./scripts/generate-badge.sh <label> <message> <color> <output-path>
# Example: ./scripts/generate-badge.sh coverage "82%" brightgreen .github/badges/coverage.svg

set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "Usage: $0 <label> <message> <color> <output-path>" >&2
  exit 1
fi

label="$1"
message="$2"
color_name="$3"
output="$4"

case "$color_name" in
  brightgreen) color="#4c1" ;;
  green)       color="#97ca00" ;;
  yellow)      color="#dfb317" ;;
  orange)      color="#fe7d37" ;;
  red)         color="#e05d44" ;;
  blue)        color="#007ec6" ;;
  lightgrey)   color="#9f9f9f" ;;
  *)           color="$color_name" ;;
esac

# Approximate text widths: 7px per char + 10px padding on each side.
# Verdana 11px is roughly 7px wide for typical glyphs.
label_w=$(( ${#label} * 7 + 10 ))
message_w=$(( ${#message} * 7 + 10 ))
total_w=$(( label_w + message_w ))

label_x=$(awk "BEGIN { printf \"%.1f\", $label_w / 2 }")
message_x=$(awk "BEGIN { printf \"%.1f\", $label_w + $message_w / 2 }")

mkdir -p "$(dirname "$output")"
cat > "$output" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" width="${total_w}" height="20">
  <linearGradient id="b" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="a"><rect width="${total_w}" height="20" rx="3" fill="#fff"/></clipPath>
  <g clip-path="url(#a)">
    <path fill="#555" d="M0 0h${label_w}v20H0z"/>
    <path fill="${color}" d="M${label_w} 0h${message_w}v20H${label_w}z"/>
    <path fill="url(#b)" d="M0 0h${total_w}v20H0z"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="${label_x}" y="15" fill="#010101" fill-opacity=".3">${label}</text>
    <text x="${label_x}" y="14">${label}</text>
    <text x="${message_x}" y="15" fill="#010101" fill-opacity=".3">${message}</text>
    <text x="${message_x}" y="14">${message}</text>
  </g>
</svg>
EOF
