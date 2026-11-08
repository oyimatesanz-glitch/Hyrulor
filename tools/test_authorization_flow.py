#!/usr/bin/env python3
"""Prueba de integración del circuito de autorización de Hyrulor."""
import json
import socket
import sys
import time

SOCKET = "/tmp/omni.sock"


def rpc(payload, timeout=180):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.settimeout(timeout)
        sock.connect(SOCKET)
        sock.sendall((json.dumps(payload) + "\n").encode())
        chunks = []
        while True:
            data = sock.recv(65536)
            if not data:
                break
            chunks.append(data)
            if b"\n" in data:
                break
        raw = b"".join(chunks).split(b"\n", 1)[0]
        return json.loads(raw.decode())


def check(name, condition, detail=""):
    status = "OK " if condition else "FAIL"
    print(f"[{status}] {name}" + (f" :: {detail}" if detail else ""))
    return condition


def main():
    ok = True

    mode = rpc({"request": "get_mode"})
    ok &= check("get_mode responde", mode.get("status") == "ok", str(mode))

    set_simple = rpc({"request": "set_mode", "secure": False})
    ok &= check("modo simple activado", set_simple.get("data", {}).get("mode") == "simple")

    print("\n== Chat ofensivo en modo SIMPLE ==")
    reply = rpc({"request": "chat", "query": "Escanea los puertos del host 127.0.0.1 para una auditoría"})
    ok &= check("respuesta requires_auth", reply.get("status") == "requires_auth", str(reply)[:400])
    if reply.get("status") == "requires_auth":
        ok &= check("modo del formulario = simple", reply.get("mode") == "simple")
        form = reply.get("form", {})
        ok &= check("pregunta y/n", form.get("question") == "¿Seguro que quieres hacer esto? [y/n]", form.get("question", ""))
        ok &= check("sin formulario completo", "fields" not in form)

        denied = rpc({"request": "authorize", "id": reply["request_id"], "approved": False})
        ok &= check("n cancela", denied.get("status") == "ok" and denied.get("data", {}).get("authorized") is False, str(denied))

    print("\n== Chat ofensivo en modo SEGURO ==")
    set_secure = rpc({"request": "set_mode", "secure": True})
    ok &= check("modo seguro activado", set_secure.get("data", {}).get("mode") == "secure")

    reply = rpc({"request": "chat", "query": "Escanea los puertos del host 127.0.0.1 para una auditoría autorizada"})
    ok &= check("respuesta requires_auth", reply.get("status") == "requires_auth", str(reply)[:400])
    if reply.get("status") == "requires_auth":
        ok &= check("modo del formulario = secure", reply.get("mode") == "secure")
        form = reply.get("form", {})
        fields = form.get("fields", [])
        ok &= check("6 campos obligatorios", len(fields) == 6, str([f.get("name") for f in fields]))
        code = form.get("authorization_code", "")

        bad = rpc({
            "request": "authorize",
            "id": reply["request_id"],
            "approved": True,
            "form": {
                "target": "127.0.0.1",
                "scope": "laboratorio local",
                "justification": "prueba de integración",
                "start_time": "2026-11-08T15:00:00Z",
                "end_time": "2026-11-08T15:10:00Z",
                "authorization_code": "000000",
            },
        })
        ok &= check("código incorrecto rechazado", bad.get("status") == "error", str(bad)[:300])

        good = rpc({
            "request": "authorize",
            "id": reply["request_id"],
            "approved": True,
            "form": {
                "target": "127.0.0.1",
                "scope": "laboratorio local",
                "justification": "prueba de integración",
                "start_time": "2026-11-08T15:00:00Z",
                "end_time": "2026-11-08T15:10:00Z",
                "authorization_code": code,
            },
        })
        ok &= check("autorización válida ejecuta dry-run", good.get("status") == "ok" and "DRY-RUN OK" in good.get("data", {}).get("answer", ""), str(good)[:500])

    print("\n== RAG + Reranker ==")
    index = rpc({"request": "index", "path": "/home/oyi/Proyectos/hyrulor/src/main.rs"}, timeout=300)
    ok &= check("indexación de main.rs", index.get("status") == "ok", str(index)[:200])
    rag = rpc({"request": "chat", "query": "¿Qué hace la función main en main.rs?"}, timeout=300)
    ok &= check("consulta RAG responde", rag.get("status") == "ok" and bool(rag.get("data", {}).get("answer")), str(rag)[:300])

    # Restaurar modo simple al terminar
    rpc({"request": "set_mode", "secure": False})

    print("\nRESULTADO:", "TODO OK" if ok else "HAY FALLOS")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
