#!/usr/bin/env python3
"""Count tokens in two OpenCode context-check captures using tiktoken."""

import argparse
import json
from pathlib import Path

import tiktoken


def serialized(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def measure(path, encoding):
    def count(text):
        return len(encoding.encode_ordinary(text))

    requests = json.loads(path.read_text())
    functions = [tool["function"] for tool in requests[0]["tools"]
                 if tool["function"]["name"].startswith("agent_job_relay_")]
    totals = {
        "model_requests": len(requests),
        "relay_tool_count": len(functions),
        "tool_descriptions": sum(count(tool["description"]) for tool in functions),
        "input_schemas": sum(count(serialized(tool["parameters"])) for tool in functions),
        "guidance_loads": 0,
        "guidance_responses": 0,
        "relay_calls": 0,
        "relay_responses": {},
        "notification": 0,
        "submission_input": 0,
        "completion_input": 0,
    }
    seen = set()
    for request in requests:
        messages, tools = request["messages"], request["tools"]
        users = [message for message in messages if message["role"] == "user"]
        notification = "AgentJobRelay event" in serialized(users[-1]["content"])
        turn = "completion_input" if notification else "submission_input"
        totals[turn] += count(serialized({"messages": messages, "tools": tools}))
        if notification:
            totals["notification"] = count(users[-1]["content"])
        calls = {call["id"]: call["function"]["name"]
                 for message in messages for call in message.get("tool_calls", [])}
        for message in messages:
            if message["role"] != "tool":
                continue
            call_id = message["tool_call_id"]
            if call_id in seen:
                continue
            seen.add(call_id)
            name = calls[call_id]
            content = message["content"]
            if name == "bash" and '"workflow_sha256"' in content:
                totals["guidance_loads"] += 1
                totals["guidance_responses"] += count(content)
            elif name.startswith("agent_job_relay_"):
                totals["relay_calls"] += 1
                totals["relay_responses"].setdefault(name, []).append(count(content))
    totals["total_input"] = totals["submission_input"] + totals["completion_input"]
    return totals


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--encoding", default="o200k_base")
    args = parser.parse_args()
    encoding = tiktoken.get_encoding(args.encoding)
    print(json.dumps({"encoding": args.encoding, "before": measure(args.before, encoding),
                      "after": measure(args.after, encoding)}, indent=2))
