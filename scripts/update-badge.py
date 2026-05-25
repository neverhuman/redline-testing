#!/usr/bin/env python3
"""
Update the <!-- jankurai-score-badge:begin/end --> section in README.md
from agent/jankurai-badge.json.

Usage: python3 scripts/update-badge.py
"""
import re
import json
import sys

BADGE_JSON = "agent/jankurai-badge.json"
README = "README.md"
MARKER_PATTERN = (
    r"<!-- jankurai-score-badge:begin -->.*?<!-- jankurai-score-badge:end -->"
)

with open(BADGE_JSON) as f:
    badge = json.load(f)

message = badge.get("message", "unknown")
color = badge.get("color", "lightgrey")

# URL-encode for the static shields.io badge
encoded = message.replace("/", "%2F").replace(" ", "%20")
badge_url = f"https://img.shields.io/badge/jankurai-{encoded}-{color}"
badge_md = f"[![Jankurai score: {message}]({badge_url})](agent/repo-score.json)"

replacement = (
    f"<!-- jankurai-score-badge:begin -->\n"
    f"{badge_md}\n"
    f"<!-- jankurai-score-badge:end -->"
)

with open(README) as f:
    content = f.read()

new_content, n = re.subn(MARKER_PATTERN, replacement, content, flags=re.DOTALL)

if n == 0:
    print(f"ERROR: markers not found in {README}", file=sys.stderr)
    sys.exit(1)

with open(README, "w") as f:
    f.write(new_content)

changed = new_content != content
print(f"Badge: {message} ({color}) — README {'updated' if changed else 'unchanged'}")
