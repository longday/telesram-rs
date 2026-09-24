#!/usr/bin/env python3
"""Build the pinned EasyPrivacy/uBlock network subset supported by WebKit."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
SOURCES = (
    ROOT / "assets/filter-sources/easyprivacy-202609241222.txt",
    ROOT / "assets/filter-sources/ublock-privacy-038b6edb.txt",
    ROOT / "assets/filter-sources/ublock-resource-abuse-038b6edb.txt",
)
OUTPUT = ROOT / "assets/managed-rules.json"
RESOURCE_TYPES = ("image", "style-sheet", "script", "font", "raw", "svg-document", "media")
TYPE_OPTIONS = {
    "image": "image",
    "script": "script",
    "stylesheet": "style-sheet",
    "css": "style-sheet",
    "font": "font",
    "media": "media",
    "xhr": "raw",
    "xmlhttprequest": "raw",
}
HOST = re.compile(r"(?:[a-z0-9_-]+\.)+[a-z0-9_-]+", re.IGNORECASE)
HOST_PATTERN = re.compile(r"\|\|((?:[a-z0-9_-]+\.)+[a-z0-9_-]+)(\^|/)(.*)", re.IGNORECASE)
REGEX_META = re.compile(r"([\\.^$+?{}\[\]|()])")
TRACKER_HOSTS = ("metrika.yandex.ru", "mc.yandex.ru", "mc.webvisor.org", "an.yandex.ru")
SERVICE_HOSTS = (
    "telemost.360.yandex.ru",
    "telemost.yandex.ru",
    "passport.yandex.ru",
    "id.yandex.ru",
    "cookier.360.yandex.ru",
)


def active_lines():
    """Skip uBO conditional branches rather than guessing browser-specific semantics."""
    lines = []
    skipped = Counter()
    for source in SOURCES:
        depth = 0
        for original in source.read_text(encoding="utf-8").splitlines():
            line = original.strip()
            if line.startswith("!#if"):
                depth += 1
                continue
            if line.startswith("!#endif"):
                if depth == 0:
                    raise ValueError(f"unmatched !#endif in {source}")
                depth -= 1
                continue
            if line.startswith("!#else"):
                if depth == 0:
                    raise ValueError(f"unmatched !#else in {source}")
                continue
            if line.startswith("!#include"):
                if line != "!#include resource-abuse.txt":
                    raise ValueError(f"unknown uBlock include: {line}")
                continue  # The pinned included file is processed separately.
            if not line or line.startswith(("!", "[")):
                continue
            if depth:
                skipped["conditional"] += 1
                continue
            lines.append(line)
        if depth:
            raise ValueError(f"unclosed !#if in {source}")
    return lines, skipped


def escaped_pattern(pattern):
    if not pattern or pattern == "*" or any(char in pattern for char in "^|$"):
        return None
    if pattern.startswith("/") and pattern.endswith("/") and len(pattern) > 2:
        return None  # uBO regular expressions cannot be translated as ABP literals.
    if len(pattern.replace("*", "")) < 4:
        return None
    return ".*".join(REGEX_META.sub(r"\\\1", part) for part in pattern.split("*"))


def url_filter(pattern):
    match = HOST_PATTERN.fullmatch(pattern)
    if match:
        host, delimiter, rest = match.groups()
        prefix = rf"^https?://([^:/]+\.)?{re.escape(host)}(:[0-9]+)?/"
        if delimiter == "^":
            return prefix if not rest else None
        suffix = escaped_pattern(rest)
        return prefix + suffix if suffix is not None else None
    if pattern.startswith("||"):
        return None  # No host boundary: could block unrelated lookalike domains.
    if HOST.fullmatch(pattern):
        return rf"^https?://([^:/]+\.)?{re.escape(pattern)}(:[0-9]+)?/"
    return escaped_pattern(pattern)


def convert(line):
    if any(marker in line for marker in ("##", "#@#", "#?#", "#$#", "#%#")):
        return None, "cosmetic"
    exception = line.startswith("@@")
    body = line[2:] if exception else line
    pattern, separator, option_text = body.partition("$")
    options = option_text.split(",") if separator else []
    types = []
    case_sensitive = False
    third_party = False
    for option in options:
        if option in TYPE_OPTIONS:
            types.append(TYPE_OPTIONS[option])
        elif option == "match-case":
            case_sensitive = True
        elif option in ("third-party", "3p"):
            third_party = True
        else:
            return None, "option:" + option.split("=", 1)[0]
    if third_party:
        host_match = HOST_PATTERN.fullmatch(pattern)
        # On Telemost/ID pages, a host-anchored non-Yandex tracker is third-party.
        # Restrict WebKit's origin-based test as well; skip broad or Yandex rules.
        if exception or not host_match or ".yandex." in f".{host_match.group(1).lower()}.":
            return None, "option:third-party"
    regex = url_filter(pattern)
    if regex is None:
        return None, "pattern"
    trigger = {
        "url-filter": regex,
        "url-filter-is-case-sensitive": case_sensitive,
        "resource-type": sorted(set(types)) if types else RESOURCE_TYPES,
    }
    if third_party:
        trigger["load-type"] = ["third-party"]
    return {
        "trigger": trigger,
        "action": {"type": "ignore-previous-rules" if exception else "block"},
    }, None


def build_rules():
    lines, skipped = active_lines()
    disabled = set()
    for line in lines:
        body, marker, options = line.partition("$")
        if marker and "badfilter" in options.split(","):
            disabled.add((body, tuple(sorted(x for x in options.split(",") if x != "badfilter"))))

    blocks = []
    exceptions = []
    seen = set()
    for line in lines:
        body, marker, options = line.partition("$")
        key = (body, tuple(sorted(options.split(",")))) if marker else (body, ())
        if key in disabled or marker and "badfilter" in options.split(","):
            skipped["badfilter"] += 1
            continue
        rule, reason = convert(line)
        if reason:
            skipped[reason] += 1
            continue
        identifier = json.dumps(rule, sort_keys=True, separators=(",", ":"))
        if identifier in seen:
            skipped["duplicate"] += 1
            continue
        seen.add(identifier)
        (exceptions if line.startswith("@@") else blocks).append(rule)

    for host in TRACKER_HOSTS:
        rule = {
            "trigger": {
                "url-filter": rf"^https?://([^:/]+\.)?{re.escape(host)}(:[0-9]+)?/",
                "url-filter-is-case-sensitive": False,
                "resource-type": RESOURCE_TYPES,
            },
            "action": {"type": "block"},
        }
        identifier = json.dumps(rule, sort_keys=True, separators=(",", ":"))
        if identifier not in seen:
            blocks.append(rule)
            seen.add(identifier)

    # A generic path filter must not disable Telemost or Yandex ID subresources.
    # Tracker requests on dedicated hosts (mc.yandex.ru, etc.) remain blocked.
    for host in SERVICE_HOSTS:
        exceptions.append(
            {
                "trigger": {
                    "url-filter": rf"^https?://{re.escape(host)}(:[0-9]+)?/",
                    "url-filter-is-case-sensitive": False,
                    "resource-type": RESOURCE_TYPES,
                },
                "action": {"type": "ignore-previous-rules"},
            }
        )

    # Exceptions must follow blocks in WebKit's ordered rule list. The CSS
    # rule comes last so a network exception cannot undo the requested hide.
    cosmetic = {
        "trigger": {"url-filter": ".*"},
        "action": {"type": "css-display-none", "selector": "div.yamb-global-bar"},
    }
    return blocks + exceptions + [cosmetic], skipped, len(blocks), len(exceptions)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify the checked-in JSON is current")
    args = parser.parse_args()
    rules, skipped, blocks, exceptions = build_rules()
    content = json.dumps(rules, ensure_ascii=True, separators=(",", ":")) + "\n"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != content:
            parser.error(f"{OUTPUT} is out of date")
    else:
        OUTPUT.write_text(content, encoding="utf-8")
    print(f"{blocks} blocks, {exceptions} exceptions, 1 CSS rule; {len(content)} bytes")
    print("skipped:", ", ".join(f"{key}={count}" for key, count in sorted(skipped.items())))


if __name__ == "__main__":
    main()
