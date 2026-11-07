#!/bin/bash
ROOT="$HOME/Proyectos/hyrulor"
LOGS="$ROOT/logs"
PIDS="$ROOT/.pids"
mkdir -p "$LOGS" "$PIDS"

LLM="$ROOT/models/Qwen2.5-3B-Instruct-Q5_K_M.gguf"
EMB="$ROOT/models/embeddings/Qwen3-Embedding-0.6B-Q8_0.gguf"
BIN="$ROOT/runtime/llama.cpp/build/bin/llama-server"
DAEMON="$ROOT/target/release/hyory"

stop() {
    for s in daemon embed llm; do
        [ -f "$PIDS/$s.pid" ] && kill "$(cat "$PIDS/$s.pid")" 2>/dev/null
        rm -f "$PIDS/$s.pid"
    done
    pkill -f "llama-server.*$ROOT" 2>/dev/null
    rm -f /tmp/omni.sock
    echo "🛑 Parado"
}

start() {
    stop; sleep 1
    nohup "$BIN" -m "$LLM" --host 127.0.0.1 --port 8082 --ctx-size 4096 --n-gpu-layers 99 --jinja > "$LOGS/llm.log" 2>&1 &
    echo $! > "$PIDS/llm.pid"
    nohup "$BIN" -m "$EMB" --host 127.0.0.1 --port 8081 --embeddings --ctx-size 2048 --n-gpu-layers 99 > "$LOGS/embed.log" 2>&1 &
    echo $! > "$PIDS/embed.pid"
    sleep 4
    cd "$ROOT"
    HYORY_SOCKET=/tmp/omni.sock HYORY_LLM_ADDR=127.0.0.1:8082 HYORY_EMBEDDING_ADDR=127.0.0.1:8081 \
    HYORY_LLM_MODEL=Qwen2.5-3B-Instruct-Q5_K_M HYORY_DB="$ROOT/hyory.db" HYORY_TOOLS="$ROOT/tools" \
    RUST_LOG=info nohup "$DAEMON" > "$LOGS/daemon.log" 2>&1 &
    echo $! > "$PIDS/daemon.pid"
    sleep 2
    echo "🚀 Arrancado. Prueba: ./hyrulor.sh chat \"hola\""
}

case "$1" in
    start) start ;;
    stop) stop ;;
    chat) shift; printf '%s\n' "{\"request\":\"chat\",\"query\":\"$*\"}" | nc -U /tmp/omni.sock ;;
    status) for s in llm embed daemon; do
        [ -f "$PIDS/$s.pid" ] && kill -0 "$(cat "$PIDS/$s.pid")" 2>/dev/null && echo "🟢 $s" || echo "🔴 $s"
    done ;;
    logs) tail -f "$LOGS/${2:-daemon}.log" ;;
    *) echo "Uso: ./hyrulor.sh {start|stop|chat \"...\"|status|logs}" ;;
esac
