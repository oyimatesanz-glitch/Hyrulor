# Prototipos Go (histórico)

Código de referencia de las primeras versiones del orquestador. **No forma
parte del daemon actual** (`src/`, Rust); se conserva para consulta.

## Variantes y build tags

Los tres prototipos declaran `func main` y los mismos tipos, así que se
compilan por separado mediante build tags mutuamente excluyentes:

| Variante | Archivos | Comando |
|---|---|---|
| Segura (por defecto) | `orchestrator_secure.go` + `types.go` | `go build ./...` |
| v1 (memoria JSON) | `orchestrator.go` + `types.go` | `go build -tags legacy_v1 ./...` |
| v2 (fragmento) | `orchestrator_v2.go` + `types.go` | `go build -tags legacy_v2 ./...` |
| Memoria JSON alternativa | `memory.go` (aislado) | `go vet -tags legacy_v1_memory ./...` |

`types.go` contiene los tipos compartidos (`Message`, `ToolCall`,
`ChatRequest`, `ChatResponse`, etc.). `memory.go` duplica `saveToMemory` y
solo se compila de forma aislada con su propio tag.

## Dependencias

La variante segura usa SQLite puro-Go:

```sh
go mod tidy
go build ./...
```

`modernc.org/sqlite` está fijado en una versión compatible con Go 1.22
(`go 1.22.2` en `go.mod`). El binario resultante (`legacy-go/orchestrator`)
está en `.gitignore`.

## Notas

- `orchestrator_secure.go` lee `schema_safe.sql` del directorio de trabajo.
- Las herramientas externas (`./battery_tool.sh`, `./tools/*.sh`) son
  referencias históricas; la implementación real vive en `tools/` y se ejecuta
  a través del sandbox del daemon Rust.
