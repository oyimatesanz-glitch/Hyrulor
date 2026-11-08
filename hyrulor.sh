#!/bin/bash
ROOT="$HOME/Proyectos/hyrulor"
LOGS="$ROOT/logs"
PIDS="$ROOT/.pids"
RUN="$HOME/.hyrulor_run"

mkdir -p "$LOGS" "$PIDS" "$RUN"
chmod 700 "$RUN" # Solo el dueño puede entrar

LLM="$ROOT/models/Qwen2.5-3B-Instruct-Q5_K_M.gguf"
EMB="$ROOT/models/embeddings/Qwen3-Embedding-0.6B-Q8_0.gguf"
CODER="$ROOT/models/specialists/Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf"
GUARD="$ROOT/models/specialists/Qwen3Guard-Gen-0.6B-heretic-Q8_0.gguf"
RERANKER="$ROOT/models/specialists/Qwen3-Reranker-0.6B-Q8_0.gguf"
DEEPSEEK="$ROOT/models/specialists/DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf"
PHI="$ROOT/models/specialists/Phi-4-mini-instruct-Q4_K_M.gguf"
BIN="$ROOT/runtime/llama.cpp/build/bin/llama-server"
DAEMON="$ROOT/target/release/hyory"

# Nombres únicos de sockets para evitar colisiones
SOCK_LLM="$RUN/llm.sock"
SOCK_EMB="$RUN/emb.sock"
SOCK_CODER="$RUN/coder.sock"
SOCK_GUARD="$RUN/guard.sock"
SOCK_RERANKER="$RUN/reranker.sock"
SOCK_DEEPSEEK="$RUN/deepseek.sock"
SOCK_PHI="$RUN/phi.sock"
SOCK_OMNI="/tmp/omni.sock"

wait_sock() {
    local sock=$1 name=$2
    for i in {1..30}; do
        if [ -S "$sock" ] && [ "$(stat -c '%a' "$sock")" = "600" ]; then
            echo "  ✅ $name listo ($sock)"
            return 0
        fi
        sleep 0.5
    done
    echo "  ⚠️  $name no creó el socket. Revisa logs."
}

start_llama() {
    local model=$1 sock=$2 name=$3 ctx=$4 extra=$5
    echo "  ⏳ Arrancando $name..."
    nohup "$BIN" -m "$model" --host "$sock" --port 9999 \
        --ctx-size "$ctx" --n-gpu-layers 99 $extra \
        > "$LOGS/$name.log" 2>&1 &
    echo $! > "$PIDS/$name.pid"
    local pid=$!

    # Esperar a que el socket exista y el motor responda /health (evita HTTP 503).
    for i in {1..60}; do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "  ❌ $name terminó durante el arranque. Últimas líneas del log:"
            tail -n 5 "$LOGS/$name.log" 2>/dev/null | sed 's/^/     /'
            return 1
        fi
        if [ -S "$sock" ]; then
            chmod 600 "$sock"
            if curl -sf --max-time 1 --unix-socket "$sock" http://localhost/health 2>/dev/null \
                | grep -q '"status"[[:space:]]*:[[:space:]]*"ok"'; then
                echo "  ✅ $name cargado y listo."
                return 0
            fi
        fi
        sleep 1
    done
    echo "  ⚠️  $name tardó demasiado en cargar. Revisa $LOGS/$name.log"
    return 1
}

stop() {
    for s in daemon phi deepseek reranker guard coder embed llm; do
        [ -f "$PIDS/$s.pid" ] && kill "$(cat "$PIDS/$s.pid")" 2>/dev/null
        rm -f "$PIDS/$s.pid"
    done
    pkill -f "llama-server.*$ROOT" 2>/dev/null
    rm -f "$SOCK_LLM" "$SOCK_EMB" "$SOCK_CODER" "$SOCK_GUARD" \
          "$SOCK_RERANKER" "$SOCK_DEEPSEEK" "$SOCK_PHI" "$SOCK_OMNI"
    echo "🛑 Parado"
}

start() {
    stop; sleep 1
    start_llama "$LLM" "$SOCK_LLM" "llm" 4096 "--jinja"
    start_llama "$EMB" "$SOCK_EMB" "embed" 2048 "--embeddings"
    
    if [ -f "$CODER" ]; then
        start_llama "$CODER" "$SOCK_CODER" "coder" 4096 "--jinja"
    else
        echo "  ⚠️ Coder no instalado."
    fi

    if [ -f "$GUARD" ]; then
        start_llama "$GUARD" "$SOCK_GUARD" "guard" 2048 ""
    fi
    if [ -f "$RERANKER" ]; then
        start_llama "$RERANKER" "$SOCK_RERANKER" "reranker" 2048 ""
    fi
    if [ -f "$DEEPSEEK" ]; then
        start_llama "$DEEPSEEK" "$SOCK_DEEPSEEK" "deepseek" 4096 "--jinja"
    fi
    if [ -f "$PHI" ]; then
        start_llama "$PHI" "$SOCK_PHI" "phi" 4096 "--jinja"
    fi

    echo "  ⏳ Arrancando Daemon (TCP interno privado)..."
    cd "$ROOT"
    # El daemon usará Unix Sockets para todo
    HYORY_SOCKET="$SOCK_OMNI" \
    HYORY_LLM_SOCKET="$SOCK_LLM" \
    HYORY_EMBEDDING_SOCKET="$SOCK_EMB" \
    HYORY_CODER_SOCKET="$SOCK_CODER" \
    HYORY_GUARD_SOCKET="$SOCK_GUARD" \
    HYORY_LLM_MODEL=Qwen2.5-3B-Instruct-Q5_K_M \
    HYORY_DB="$ROOT/hyory.db" \
    HYORY_TOOLS="$ROOT/tools" \
    RUST_LOG=info nohup "$DAEMON" > "$LOGS/daemon.log" 2>&1 &
    echo $! > "$PIDS/daemon.pid"
    
    wait_sock "$SOCK_OMNI" "Daemon"
    echo ""
    echo "🎉 Todo listo. Comandos:"
    echo "   ./hyrulor.sh chat \"tu pregunta\""
    echo "   ./hyrulor.sh logs [llm|embed|coder|guard|daemon]"
}

chat() {
    if [ ! -S "$SOCK_OMNI" ]; then
        echo "❌ Daemon no está corriendo. Usa: ./hyrulor.sh start"
        exit 1
    fi
    printf '%s\n' "{\"request\":\"chat\",\"query\":\"$1\"}" | nc -U "$SOCK_OMNI"
}

case "$1" in
    start) start ;;
    stop) stop ;;
    restart) stop; sleep 1; start ;;
    chat) shift; chat "$*" ;;
    status)
        echo "Estado de Sockets Privados:"
        for s in llm embed coder guard reranker deepseek phi; do
            [ -f "$PIDS/$s.pid" ] && kill -0 "$(cat "$PIDS/$s.pid")" 2>/dev/null && echo "  🟢 $s" || echo "  🔴 $s"
        done
        [ -f "$PIDS/daemon.pid" ] && kill -0 "$(cat "$PIDS/daemon.pid")" 2>/dev/null && echo "  🟢 daemon" || echo "  🔴 daemon"
        ;;
    logs) tail -f "$LOGS/${2:-daemon}.log" ;;
    *) echo "Uso: ./hyrulor.sh {start|stop|restart|chat|status|logs}" ;;
esac
