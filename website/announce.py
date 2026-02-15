#!/usr/bin/env python3
"""Send a blog post announcement to the tesseras-announce mailing list."""

import argparse
import re
import subprocess
import sys
import textwrap
import tomllib
from pathlib import Path

FROM = "ijanc <ijanc@ijanc.org>"
TO = "~ijanc/tesseras-announce@lists.sr.ht"
BASE_URL = "https://tesseras.net/news"
FILENAME_RE = re.compile(r"^\d{4}-\d{2}-\d{2}-(.+)\.md$")


def parse_front_matter(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    parts = text.split("+++", maxsplit=2)
    if len(parts) < 3:
        sys.exit(f"error: could not find +++ delimited front matter in {path}")
    return tomllib.loads(parts[1])


def slug_from_filename(path: Path) -> str:
    m = FILENAME_RE.match(path.name)
    if not m:
        sys.exit(
            f"error: filename does not match YYYY-MM-DD-<slug>.md pattern: {path.name}"
        )
    return m.group(1)


def wrap(text: str) -> str:
    """Wrap text at 72 columns per mailing list etiquette."""
    return textwrap.fill(text, width=72)


def compose_email(title: str, description: str, url: str) -> str:
    body = wrap(description)
    return (
        f"From: {FROM}\n"
        f"Subject: {title}\n"
        f"To: {TO}\n"
        f"Content-Type: text/plain; charset=utf-8\n"
        f"\n"
        f"{title}\n"
        f"\n"
        f"{body}\n"
        f"\n"
        f"Read the full post:\n"
        f"{url}\n"
        f"\n"
        f"-- \n"
        f"Tesseras — P2P network for preserving human memories\n"
        f"https://tesseras.net\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("post", type=Path, help="path to blog post markdown file")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print composed email to stdout instead of sending",
    )
    args = parser.parse_args()

    if not args.post.exists():
        sys.exit(f"error: file not found: {args.post}")

    front_matter = parse_front_matter(args.post)
    title = front_matter.get("title")
    description = front_matter.get("description")

    if not title:
        sys.exit("error: front matter missing 'title'")
    if not description:
        sys.exit("error: front matter missing 'description'")

    slug = slug_from_filename(args.post)
    url = f"{BASE_URL}/{slug}/"
    email = compose_email(title, description, url)

    if args.dry_run:
        print(email)
        return

    result = subprocess.run(
        ["msmtp", "-t"],
        input=email.encode("utf-8"),
        check=False,
    )
    if result.returncode != 0:
        sys.exit(f"error: msmtp exited with code {result.returncode}")

    print(f"Sent announcement to {TO}: {title}")


if __name__ == "__main__":
    main()
