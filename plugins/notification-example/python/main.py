import json
import sys

# 把这里改成你要测试的文件路径，例如：
# ACTION_PATH = r"C:\Users\admin\Desktop\demo.txt"
ACTION_PATH = r"C:\Users\admin\Downloads\320495704b4b4db6dfa5ecd06ae8e1ef.png"


def python_notification(text, file, config):
    data = "Python 能力执行完成"

    # close 是不需要路径的内置 Action，可以直接使用。
    actions = [
        {"type": "close", "label": "知道了"},
    ]

    # 填写 ACTION_PATH 后，会额外显示“打开文件”按钮。
    # 留空时不添加 open Action，避免因为空路径导致通知校验失败。
    if ACTION_PATH.strip():
        actions.append({
            "type": "open",
            "path": ACTION_PATH,
            "label": "打开文件",
        })

    return {
        "result": {"title": data, "data": data, "desc": "Python 通知 Action 示例"},
        "notification": {
            "level": "info",
            "title": "Python 通知 Action 示例",
            "message": data,
            "durationMs": 10000,
            "actions": actions,
        },
    }


handlers = {"pythonNotification": python_notification}

for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    try:
        handler = handlers[request["task"]]
        args = request.get("args", {})
        response = {"id": request.get("id"), "ok": True, **handler(
            args.get("text", ""), args.get("file"), args.get("config") or {}
        )}
    except Exception as error:
        response = {"id": request.get("id"), "ok": False, "error": str(error)}
    print(json.dumps(response, ensure_ascii=False), flush=True)
