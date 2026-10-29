#!/usr/bin/python3
"""Verified replacement runtime fixture, not an actual host or job runner."""
import json
import os
from pathlib import Path
import sys
import time

if sys.argv[-1] == "instructions":
    print(json.dumps({"format": 1, "version": "0.1.11", "workflow": "current-runtime-workflow"}))
    sys.exit(0)
assert "mcp" in sys.argv and "--worker" in sys.argv and "--shared-host" in sys.argv
assert not os.environ.get("CODEX_THREAD_ID")
assert not os.environ.get("CODEX_SESSION_ID")
assert not os.environ.get("CODEX_APP_TOOLS_PIPE_PATH")
state = sys.argv[sys.argv.index("--state-dir") + 1]
for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    if request["method"] == "server/discover":
        meta=request["params"]["_meta"]
        client_context={"protocolVersion":meta["io.modelcontextprotocol/protocolVersion"],"capabilities":meta["io.modelcontextprotocol/clientCapabilities"]}
        result={"resultType":"discovery","supportedVersions":["2026-07-28"],"capabilities":{"tools":{}},"ttlMs":0,"cacheScope":"private"}
    elif request["method"] == "initialize":
        client_context = request["params"]
        result = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "agent-job-relay", "version": "0.1.11"}}
    elif request["method"] == "tools/list":
        result = {"tools": [{"name": "job_current_fixture", "inputSchema": {"type": "object"}}]}
    else:
        assert request["method"] == "tools/call"
        arguments = request["params"]["arguments"]
        if arguments.get("mrtr"):
            assert client_context["protocolVersion"] == "2026-07-28"
            assert request["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"]["elicitation"]["form"] == {}
            if "inputResponses" in request["params"]:
                assert request["params"]["requestState"] == "scoped-native-question-token"
                result={"content":[{"type":"text","text":json.dumps({"inputResponses":request["params"]["inputResponses"],"arguments":arguments,"state":state})}]}
            else:
                result={"resultType":"input_required","inputRequests":{"scope":{"method":"elicitation/create","params":{"mode":"form","message":"Allow scoped fixture?","requestedSchema":{"type":"object","properties":{"allow":{"type":"boolean"}},"required":["allow"]}}}},"requestState":"scoped-native-question-token"}
            print(json.dumps({"jsonrpc":"2.0","id":request["id"],"result":result}),flush=True)
            continue
        if arguments.get("question"):
            assert request["params"]["name"] == "job_goal_consent"
            assert client_context["capabilities"]["elicitation"]["form"] == {}
            print(json.dumps({"jsonrpc":"2.0", "id":"worker-user-question", "method":"elicitation/create", "params":{"mode":"form", "message":"Allow this scoped test?", "requestedSchema":{"type":"object", "properties":{"allow":{"type":"boolean"}}, "required":["allow"]}}}),flush=True)
            answer=json.loads(sys.stdin.readline())
            assert answer["id"] == "worker-user-question"
            result={"content":[{"type":"text","text":json.dumps({"answer":answer.get("result"),"client_protocol":client_context["protocolVersion"],"state":state})}]}
            print(json.dumps({"jsonrpc":"2.0","id":request["id"],"result":result}),flush=True)
            continue
        if arguments.get("hold"):
            Path(arguments["entered"]).touch()
            deadline = time.monotonic() + 8
            while not Path(arguments["release"]).exists():
                assert time.monotonic() < deadline
                time.sleep(.01)
        if arguments.get("drop_ack"):
            with Path(arguments["record"]).open("a") as output:
                output.write(arguments["idempotency_key"] + "\n")
            sys.exit(0)
        result = {"content": [{"type": "text", "text": json.dumps({"runtime": "current", "state": state, "arguments": arguments})}]}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
