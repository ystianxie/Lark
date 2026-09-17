#!/usr/bin/env python3
"""Minimal JSON-lines runner used by Lark Python plugins."""
import json
import sys


def main():
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            request = json.loads(line)
            task = request.get("task")
            args = request.get("args", {})
            # Plugin runners may replace this dispatcher with their own tasks.
            result = {"type": "result", "task": task, "data": args}
            print(json.dumps({"id": request.get("id"), "ok": True, "result": result}, ensure_ascii=False), flush=True)
        except Exception as exc:
            print(json.dumps({"id": None, "ok": False, "error": str(exc)}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
