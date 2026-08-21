#!/usr/bin/env python3
"""Validate every schemas/examples/*.json against the schema it names.

Nothing in the build enforces the JSON Schemas today: there is no jsonschema,
valico, or schemars dependency, and tests/test_schema_json_suite.rs parses four
of the six examples as untyped serde_json::Value. This script is the missing
check, kept out of the test suite deliberately -- it currently reports failures,
and turning it into a gate is a decision recorded in docs/GEOMETRY_AND_SCHEMA.md.

Usage:  python3 scripts/validate_schemas.py [--quiet]
Exit:   0 if every example conforms, 1 otherwise.
"""

import json
import sys
from pathlib import Path

try:
    import jsonschema
except ImportError:
    sys.exit("jsonschema is required: pip install jsonschema")

SCHEMAS = Path(__file__).resolve().parent.parent / "schemas"
EXAMPLES = SCHEMAS / "examples"


def schema_for(example: Path) -> Path:
    """Resolve the schema an example declares via its own $schema key.

    Falls back to the sample_<name>.json naming convention when $schema points
    somewhere that is not one of ours.
    """
    declared = json.loads(example.read_text()).get("$schema", "")
    candidate = SCHEMAS / Path(declared).name
    if candidate.is_file():
        return candidate
    return SCHEMAS / example.name.removeprefix("sample_")


def main() -> int:
    quiet = "--quiet" in sys.argv
    failures = 0

    for example in sorted(EXAMPLES.glob("*.json")):
        schema_path = schema_for(example)
        if not schema_path.is_file():
            print(f"FAIL  {example.name}  -> no schema found")
            failures += 1
            continue

        schema = json.loads(schema_path.read_text())
        instance = json.loads(example.read_text())
        errors = sorted(
            jsonschema.Draft7Validator(schema).iter_errors(instance),
            key=lambda e: list(e.path),
        )

        if not errors:
            if not quiet:
                print(f"ok    {example.name}  -> {schema_path.name}")
            continue

        failures += 1
        print(f"FAIL  {example.name}  -> {schema_path.name}  ({len(errors)} errors)")
        for err in errors:
            where = "/".join(str(p) for p in err.path) or "(root)"
            print(f"        {where}: {err.message}")

    print(f"\n{failures} of {len(list(EXAMPLES.glob('*.json')))} examples fail their schema.")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
