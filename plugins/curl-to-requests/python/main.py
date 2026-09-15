import json
import shlex
import sys
from urllib.parse import parse_qsl, urlsplit, urlunsplit


def parse_curl(command):
    tokens = shlex.split(command, posix=True)
    if tokens and tokens[0].lower() == "curl":
        tokens = tokens[1:]

    method = None
    url = None
    headers = []
    cookies = []
    data = None
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token in ("-X", "--request") and index + 1 < len(tokens):
            method = tokens[index + 1].upper()
            index += 2
            continue
        if token in ("-H", "--header") and index + 1 < len(tokens):
            headers.append(tokens[index + 1])
            index += 2
            continue
        if token in ("-b", "--cookie") and index + 1 < len(tokens):
            cookies.append(tokens[index + 1])
            index += 2
            continue
        if token in ("-d", "--data", "--data-raw", "--data-binary") and index + 1 < len(tokens):
            data = tokens[index + 1]
            index += 2
            continue
        if token in ("--url",) and index + 1 < len(tokens):
            url = tokens[index + 1]
            index += 2
            continue
        if not token.startswith("-") and url is None:
            url = token
        index += 1

    if not url:
        raise ValueError("没有找到 URL")
    if method is None:
        method = "POST" if data is not None else "GET"
    return method, url, headers, cookies, data


def _split_request_url(raw_url):
    parsed = urlsplit(raw_url)
    params = parse_qsl(parsed.query, keep_blank_values=True)
    url = urlunsplit((parsed.scheme, parsed.netloc, parsed.path, "", parsed.fragment))
    return url, params


def _parse_cookies(cookie_values, headers):
    pairs = []
    for value in cookie_values:
        pairs.extend(part.strip() for part in value.split(";") if part.strip())
    remaining_headers = []
    for header in headers:
        if ":" in header and header.split(":", 1)[0].strip().lower() == "cookie":
            pairs.extend(part.strip() for part in header.split(":", 1)[1].split(";") if part.strip())
        else:
            remaining_headers.append(header)
    cookies = []
    for pair in pairs:
        if "=" in pair:
            key, value = pair.split("=", 1)
            cookies.append((key.strip(), value.strip()))
    return remaining_headers, cookies


def to_python(command):
    method, raw_url, headers, cookie_values, data = parse_curl(command)
    url, params = _split_request_url(raw_url)
    headers, cookies = _parse_cookies(cookie_values, headers)
    lines = ["import json", "import requests", ""]
    if headers:
        lines.append("headers = {")
        for header in headers:
            if ":" in header:
                key, value = header.split(":", 1)
                lines.append(f"    {key.strip()!r}: {value.strip()!r},")
        lines.append("}")
    else:
        lines.append("headers = {}")
    if cookies:
        lines.append("cookies = {")
        for key, value in cookies:
            lines.append(f"    {key!r}: {value!r},")
        lines.append("}")
    else:
        lines.append("cookies = {}")
    if data is not None:
        try:
            parsed = json.loads(data)
            lines.append(f'data = json.dumps({parsed!r}, separators=(",", ":"))')
        except json.JSONDecodeError:
            lines.append(f"data = {data!r}")
    else:
        lines.append("data = None")
    lines.append(f"url = {url!r}")
    lines.append(f"params = {params!r}")
    arguments = "headers=headers, cookies=cookies, data=data, params=params"
    lines.append(f"response = requests.{method.lower()}(url, {arguments})")
    lines.extend(["print(response.status_code)", "print(response.text)"])
    return "\n".join(lines)


def main():
    for line in sys.stdin:
        if not line.strip():
            continue
        request = json.loads(line)
        try:
            if request.get("task") != "convert":
                raise ValueError("不支持的任务")
            code = to_python(request.get("args", {}).get("curl", ""))
            response = {"id": request.get("id"), "ok": True, "result": {"code": code}}
        except Exception as error:
            response = {"id": request.get("id"), "ok": False, "error": str(error)}
        print(json.dumps(response, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
