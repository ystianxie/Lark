import http.client
import json
import socket
import ssl
import sys
import time
from urllib.parse import urlencode, urlsplit, urlunsplit

MAX_BODY_BYTES = 8 * 1024 * 1024
MAX_RESPONSE_BYTES = 8 * 1024 * 1024
TIMEOUT_SECONDS = 30
METHODS = {"GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"}


def enabled_pairs(items):
    result = []
    for item in items or []:
        if not isinstance(item, dict) or item.get("enabled") is False:
            continue
        key = str(item.get("key", "")).strip()
        if key:
            result.append((key, str(item.get("value", ""))))
    return result


def build_request(args):
    method = str(args.get("method", "GET")).upper()
    if method not in METHODS:
        raise ValueError("不支持的 HTTP 方法")
    raw_url = str(args.get("url", "")).strip()
    parts = urlsplit(raw_url)
    if parts.scheme not in ("http", "https") or not parts.hostname:
        raise ValueError("URL 必须是完整的 http:// 或 https:// 地址")
    if parts.username or parts.password:
        raise ValueError("请勿将账号密码写在 URL 中")
    params = enabled_pairs(args.get("params"))
    query = parts.query
    extra_query = urlencode(params, doseq=True)
    if extra_query:
        query = f"{query}&{extra_query}" if query else extra_query
    path = urlunsplit(("", "", parts.path or "/", query, ""))

    headers = {}
    for key, value in enabled_pairs(args.get("headers")):
        if "\r" in key or "\n" in key or "\r" in value or "\n" in value:
            raise ValueError("Header 不允许包含换行符")
        headers[key] = value
    cookies = enabled_pairs(args.get("cookies"))
    if cookies:
        cookie_value = "; ".join(f"{key}={value}" for key, value in cookies)
        existing = next((key for key in headers if key.lower() == "cookie"), None)
        if existing:
            headers[existing] = f"{headers[existing]}; {cookie_value}"
        else:
            headers["Cookie"] = cookie_value

    body_type = str(args.get("bodyType", "none"))
    body = None
    if method not in ("GET", "HEAD"):
        if body_type == "json":
            text = str(args.get("body", ""))
            try:
                json.loads(text or "null")
            except json.JSONDecodeError as error:
                raise ValueError(f"JSON 请求体格式无效：{error.msg}（第 {error.lineno} 行）") from None
            body = text.encode("utf-8")
            headers.setdefault("Content-Type", "application/json;charset=UTF-8")
        elif body_type == "form":
            body = urlencode(enabled_pairs(args.get("bodyParams")), doseq=True).encode("utf-8")
            headers.setdefault("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
        elif body_type == "text":
            body = str(args.get("body", "")).encode("utf-8")
            if body:
                headers.setdefault("Content-Type", "text/plain;charset=UTF-8")
        elif body_type != "none":
            raise ValueError("不支持的 Body 类型")
    if body is not None and len(body) > MAX_BODY_BYTES:
        raise ValueError("请求体超过 8 MiB 限制")
    return method, parts, path, headers, body


def request(args):
    method, parts, path, headers, body = build_request(args)
    port = parts.port or (443 if parts.scheme == "https" else 80)
    connection_type = http.client.HTTPSConnection if parts.scheme == "https" else http.client.HTTPConnection
    connection = connection_type(parts.hostname, port, timeout=TIMEOUT_SECONDS, **({"context": ssl.create_default_context()} if parts.scheme == "https" else {}))
    started = time.perf_counter()
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        chunks = []
        total = 0
        while True:
            chunk = response.read(min(65536, MAX_RESPONSE_BYTES + 1 - total))
            if not chunk:
                break
            chunks.append(chunk)
            total += len(chunk)
            if total > MAX_RESPONSE_BYTES:
                raise ValueError("响应体超过 8 MiB 限制，已停止读取")
        raw_body = b"".join(chunks)
        charset = "utf-8"
        content_type = response.getheader("Content-Type", "")
        for part in content_type.split(";")[1:]:
            name, sep, value = part.strip().partition("=")
            if sep and name.lower() == "charset":
                charset = value.strip('"\'') or "utf-8"
                break
        try:
            text = raw_body.decode(charset, errors="replace")
        except LookupError:
            text = raw_body.decode("utf-8", errors="replace")
        return {
            "status": response.status,
            "statusText": response.reason or "",
            "elapsedMs": round((time.perf_counter() - started) * 1000),
            "headers": response.getheaders(),
            "body": text if text else "（空响应）",
            "truncated": False,
        }
    except (socket.timeout, TimeoutError):
        raise ValueError(f"请求超时（{TIMEOUT_SECONDS} 秒）") from None
    except ssl.SSLError as error:
        raise ValueError(f"TLS/证书错误：{error}") from None
    except (OSError, http.client.HTTPException) as error:
        raise ValueError(f"网络请求失败：{error}") from None
    finally:
        connection.close()


for line in sys.stdin:
    if not line.strip():
        continue
    request_data = {}
    try:
        request_data = json.loads(line)
        if request_data.get("task") != "request":
            raise ValueError("不支持的任务")
        result = request(request_data.get("args") or {})
        response_data = {"id": request_data.get("id"), "ok": True, "result": result}
    except Exception as error:
        response_data = {"id": request_data.get("id"), "ok": False, "error": str(error)}
    print(json.dumps(response_data, ensure_ascii=True), flush=True)
