import datetime as dt
import json
import sys
import time


def parse_timestamp(value):
    value = value.strip()
    if not value:
        return None
    number = float(value)
    # 13 位通常是毫秒，统一转换为 Unix 秒。
    if abs(number) >= 1e11:
        number /= 1000
    return dt.datetime.fromtimestamp(number).astimezone()


def result(title, value, description=""):
    return {"title": title, "data": str(value), "desc": description}


def convert(value):
    now = dt.datetime.now().astimezone()
    items = [
        result(int(time.time()), int(time.time()),'当前时间戳'),
        result(now.strftime("%H:%M:%S"),now.strftime("%H:%M:%S"),'当前时间'),
        result(now.strftime("%Y-%m-%d %H:%M:%S"),now.strftime("%Y-%m-%d %H:%M:%S"),"当前日期"),
    ]
    if value.strip():
        converted = parse_timestamp(value)
        items.insert(0, result(converted.strftime("%Y-%m-%d %H:%M:%S"),converted.strftime("%Y-%m-%d %H:%M:%S"),'输入时间戳'))
    return items


for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    try:
        if request.get("task") != "convert":
            raise ValueError("不支持的任务")
        items = convert(request.get("args", {}).get("text", ""))
        response = {"id": request.get("id"), "ok": True, "result": {"items": items}}
    except Exception as error:
        response = {"id": request.get("id"), "ok": False, "error": str(error)}
    # 使用 ASCII 转义避免 Windows 管道编码差异；宿主 JSON 解析后仍得到正常 Unicode。
    print(json.dumps(response, ensure_ascii=True), flush=True)
