#!/usr/bin/env python3
"""Exercises the applet JSON-lines protocol against the mock daemon.

Run the mock daemon first (it honours HYORY_SOCKET):

    HYORY_SOCKET=/tmp/hyrulor-mock.sock \
      ./target/release/hyrulor-mock-daemon &
    HYORY_SOCKET=/tmp/hyrulor-mock.sock python3 test_protocol.py
"""

import json
import os
import socket
import sys

SOCKET_PATH = os.environ.get("HYORY_SOCKET", "/tmp/hyrulor-mock.sock")


def rpc(payload):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.settimeout(30)
        sock.connect(SOCKET_PATH)
        sock.sendall((json.dumps(payload) + "\n").encode())
        chunks = []
        while True:
            data = sock.recv(65536)
            if not data:
                break
            chunks.append(data)
            if b"\n" in data:
                break
    return json.loads(b"".join(chunks).split(b"\n", 1)[0].decode())


def check(name, condition, detail=""):
    print(f"[{'OK ' if condition else 'FAIL'}] {name}" + (f" :: {detail}" if detail else ""))
    return condition


def main():
    ok = True

    mode = rpc({"request": "get_mode"})
    ok &= check("get_mode", mode.get("data", {}).get("mode") == "simple", str(mode))

    reply = rpc({"request": "chat", "query": "Escanea los puertos de 127.0.0.1"})
    ok &= check("simple requires_auth", reply.get("status") == "requires_auth", str(reply)[:300])
    ok &= check("simple mode", reply.get("mode") == "simple")
    ok &= check("simple has no fields", "fields" not in reply.get("form", {}))
    simple_id = reply.get("request_id", "")

    denied = rpc({"request": "authorize", "id": simple_id, "approved": False})
    ok &= check(
        "cancel executes the denied path",
        denied.get("status") == "ok"
        and denied.get("data", {}).get("authorized") is False,
        str(denied)[:300],
    )

    secure = rpc({"request": "set_mode", "secure": True})
    ok &= check("set_mode secure", secure.get("data", {}).get("mode") == "secure", str(secure))

    reply = rpc({"request": "chat", "query": "Haz un pentest autorizado de 127.0.0.1"})
    form = reply.get("form", {})
    fields = form.get("fields", [])
    ok &= check("secure requires_auth", reply.get("status") == "requires_auth", str(reply)[:300])
    ok &= check("secure mode", reply.get("mode") == "secure")
    ok &= check("six required fields", len(fields) == 6, str([f.get("name") for f in fields]))
    ok &= check("authorization code shown", bool(form.get("authorization_code")))

    approved = rpc(
        {
            "request": "authorize",
            "id": reply.get("request_id", ""),
            "approved": True,
            "form": {
                "target": "127.0.0.1",
                "scope": "laboratorio",
                "justification": "prueba",
                "start_time": "2026-11-08T00:00:00Z",
                "end_time": "2026-11-08T01:00:00Z",
                "authorization_code": form.get("authorization_code", ""),
            },
        }
    )
    ok &= check(
        "approval executes the authorized path",
        approved.get("status") == "ok"
        and approved.get("data", {}).get("authorized") is True,
        str(approved)[:300],
    )

    image = rpc(
        {
            "request": "describe_image",
            "path": "/tmp/foto.png",
            "prompt": "Describe la imagen",
        }
    )
    answer = image.get("data", {}).get("answer", "").lower()
    ok &= check(
        "vision answer",
        image.get("status") == "ok" and ("analizada" in answer or "analyzed" in answer),
        str(image)[:300],
    )

    transcription = rpc({"request": "listen", "seconds": 1})
    ok &= check(
        "speech to text",
        transcription.get("status") == "ok"
        and bool(transcription.get("data", {}).get("text")),
        str(transcription)[:300],
    )

    speech = rpc({"request": "speak", "text": "Hola"})
    ok &= check(
        "text to speech",
        speech.get("status") == "ok"
        and speech.get("data", {}).get("played") is True,
        str(speech)[:300],
    )

    rpc({"request": "set_mode", "secure": False})
    print("\nRESULTADO:", "TODO OK" if ok else "HAY FALLOS")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
