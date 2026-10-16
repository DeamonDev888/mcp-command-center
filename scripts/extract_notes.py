"""Extrait la section du CHANGELOG pour la version taguée (notes de release)."""
import os
import re

version = os.environ.get("GITHUB_REF_NAME", "").lstrip("v") or "0.0.0"
text = open("CHANGELOG.md", encoding="utf-8").read()
m = re.search(rf"## \[{re.escape(version)}\].*?(?=\n## \[|\Z)", text, re.S)
print(m.group(0).strip() if m else f"Release {version}")
