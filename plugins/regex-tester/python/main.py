import json
import re
import sys


FLAG_MAP = {
    "i": re.IGNORECASE,
    "m": re.MULTILINE,
    "s": re.DOTALL,
    "x": re.VERBOSE,
    "a": re.ASCII,
}
MAX_MATCHES = 500


def find_matches(pattern, text, flags):
    # Use integer 0 so this script also runs on Python versions before 3.11.
    parsed_flags = 0
    seen = set()
    for flag in flags:
        if flag in seen:
            raise ValueError(f"重复的正则标志：{flag}")
        seen.add(flag)
        if flag not in FLAG_MAP:
            raise ValueError(f"Python re 不支持标志：{flag}")
        parsed_flags |= FLAG_MAP[flag]

    expression = re.compile(pattern, parsed_flags)
    matches = []
    truncated = False
    for match in expression.finditer(text):
        if len(matches) >= MAX_MATCHES:
            truncated = True
            break
        matches.append({
            "match": match.group(0),
            "index": match.start(),
            "groups": list(match.groups()),
            "named": match.groupdict(),
        })
    return {"matches": matches, "truncated": truncated}


for line in sys.stdin:
    if not line.strip():
        continue
    request = {}
    try:
        request = json.loads(line)
        if request.get("task") != "find_matches":
            raise ValueError("不支持的任务")
        args = request.get("args", {})
        result = find_matches(
            args.get("pattern", ""), args.get("text", ""), args.get("flags", "")
        )
        response = {"id": request.get("id"), "ok": True, "result": result}
    except Exception as error:
        response = {"id": request.get("id"), "ok": False, "error": str(error)}
    print(json.dumps(response, ensure_ascii=True), flush=True)
