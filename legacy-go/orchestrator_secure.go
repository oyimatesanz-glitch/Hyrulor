package main

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
	"unicode"

	_ "modernc.org/sqlite"
)

// ... [Structs Message, ToolCall, etc. iguales que antes] ...
type Message struct {
	Role       string     `json:"role"`
	Content    string     `json:"content,omitempty"`
	ToolCalls  []ToolCall `json:"tool_calls,omitempty"`
	ToolCallID string     `json:"tool_call_id,omitempty"`
}
type ToolCall struct { ID string `json:"id"`; Type string `json:"type"`; Function ToolFunction `json:"function"` }
type ToolFunction struct { Name string `json:"name"`; Arguments string `json:"arguments"` }
type ToolDef struct { Type string `json:"type"`; Function FunctionDefinition `json:"function"` }
type FunctionDefinition struct { Name string `json:"name"`; Description string `json:"description"`; Parameters map[string]interface{} `json:"parameters"` }
type ChatRequest struct { Model string `json:"model"`; Messages []Message `json:"messages"`; Tools []ToolDef `json:"tools,omitempty"`; Temperature float64 `json:"temperature"` }
type ChatResponse struct { Choices []struct { Message Message `json:"message"`; FinishReason string `json:"finish_reason"` } `json:"choices"` }

var db *sql.DB
var binDir string

func init() {
	exe, _ := os.Executable()
	binDir = filepath.Dir(exe)
}

func initDB() error {
	var err error
	db, err = sql.Open("sqlite", "memory.db")
	if err != nil { return err }
	
	schema, _ := os.ReadFile("schema_safe.sql")
	_, err = db.Exec(string(schema))
	return err
}

func saveMemory(u, a string) {
	ts := time.Now().Format(time.RFC3339)
	_, err := db.Exec("INSERT INTO convs (ts, user, assistant) VALUES (?, ?, ?)", ts, u, a)
	if err != nil { fmt.Printf("⚠️ Error guardando memoria: %v\n", err) }
	else { fmt.Println("💾 Memoria guardada (Trigger FTS5 automático)") }
}

func buildFTSQuery(query string) string {
	cleaned := strings.Map(func(r rune) rune {
		if unicode.IsLetter(r) || unicode.IsDigit(r) || r == ' ' { return r }
		return ' '
	}, strings.ToLower(query))
	
	var terms []string
	for _, w := range strings.Fields(cleaned) {
		if len([]rune(w)) >= 3 {
			terms = append(terms, `"`+w+`"`)
		}
	}
	if len(terms) == 0 { return "" }
	return strings.Join(terms, " OR ")
}

func getRelevantContext(query string) string {
	match := buildFTSQuery(query)
	if match == "" { return "" }
	
	rows, err := db.Query(`SELECT c.user, c.assistant FROM conv_fts f JOIN convs c ON c.id=f.rowid WHERE conv_fts MATCH ? ORDER BY rank LIMIT 3`, match)
	if err != nil { return "" }
	defer rows.Close()
	
	var sb strings.Builder
	count := 0
	for rows.Next() {
		var u, a string
		rows.Scan(&u, &a)
		if count == 0 { sb.WriteString("## Contexto Relevante:\n") }
		if len(u) > 300 { u = u[:300] + "..." }
		if len(a) > 500 { a = a[:500] + "..." }
		sb.WriteString(fmt.Sprintf("- U: %s\n- A: %s\n", u, a))
		count++
	}
	out := sb.String()
	if len(out) > 2000 { out = out[:2000] + "\n...[truncado]" }
	return out
}

func callLLM(msgs []Message, tools []ToolDef) (ChatResponse, error) {
	body, _ := json.Marshal(ChatRequest{Model: "Qwen2.5-3B-Instruct-Q4_K_M", Messages: msgs, Tools: tools, Temperature: 0.1})
	req, _ := http.NewRequest("POST", "http://localhost:8080/v1/chat/completions", bytes.NewBuffer(body))
	req.Header.Set("Content-Type", "application/json")
	
	client := &http.Client{Timeout: 120 * time.Second}
	res, err := client.Do(req)
	if err != nil { return ChatResponse{}, err }
	defer res.Body.Close()
	
	if res.StatusCode != 200 {
		b, _ := io.ReadAll(res.Body)
		return ChatResponse{}, fmt.Errorf("HTTP %d: %s", res.StatusCode, string(b))
	}
	
	var resp ChatResponse
	if err := json.NewDecoder(res.Body).Decode(&resp); err != nil { return ChatResponse{}, err }
	if len(resp.Choices) == 0 { return ChatResponse{}, fmt.Errorf("sin choices") }
	return resp, nil
}

func execTool(name string) string {
	scripts := map[string]string{
		"battery_status": "tools/battery_tool.sh",
		"gpu_diag":       "tools/gpu_diag.sh",
		"net_privacy":    "tools/net_privacy.sh",
	}
	rel, ok := scripts[name]
	if !ok { return fmt.Sprintf(`{"error":"herramienta desconocida: %s"}`, name) }
	
	scriptPath := filepath.Join(binDir, rel)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	
	cmd := exec.CommandContext(ctx, "bwrap",
		"--ro-bind", "/usr", "/usr", "--ro-bind", "/lib", "/lib", "--ro-bind", "/lib64", "/lib64",
		"--ro-bind", "/bin", "/bin", "--ro-bind", "/etc", "/etc",
		"--bind", filepath.Join(binDir, "tools"), "/tools",
		"--tmpfs", "/tmp", "--proc", "/proc", "--dev", "/dev",
		"--unshare-all", "--share-net", "--die-with-parent",
		scriptPath)
	
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	
	if err := cmd.Start(); err != nil { return fmt.Sprintf(`{"error":"start: %v"}`, err) }
	
	go func() {
		<-ctx.Done()
		if cmd.Process != nil { syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) }
	}()
	
	err := cmd.Wait()
	if ctx.Err() == context.DeadlineExceeded { return `{"error":"timeout (grupo eliminado)"}` }
	if err != nil { return fmt.Sprintf(`{"error":"%v","stderr":%q}`, err, stderr.String()) }
	
	out := stdout.String()
	if len(out) > 4000 { out = out[:4000] + "...[truncado]" }
	return strings.TrimSpace(out)
}

func main() {
	fmt.Println("🤖 SuperAsistente Seguro v4.0 (SQLite+Triggers+Bwrap+Cgroups)")
	if err := initDB(); err != nil { panic(err) }

	tools := []ToolDef{
		{Type: "function", Function: FunctionDefinition{Name: "battery_status", Description: "Estado batería", Parameters: map[string]interface{}{"type": "object"}}},
		{Type: "function", Function: FunctionDefinition{Name: "gpu_diag", Description: "Diagnóstico GPU", Parameters: map[string]interface{}{"type": "object"}}},
		{Type: "function", Function: FunctionDefinition{Name: "net_privacy", Description: "Red y privacidad", Parameters: map[string]interface{}{"type": "object"}}},
	}

	userQuery := "¿Cómo está mi batería?"
	ctx := getRelevantContext(userQuery)
	sysPrompt := "Eres un asistente útil. Usa herramientas cuando sea necesario."
	if ctx != "" { sysPrompt += "\n\n" + ctx }

	msgs := []Message{{Role: "system", Content: sysPrompt}, {Role: "user", Content: userQuery}}
	resp, err := callLLM(msgs, tools)
	if err != nil { fmt.Printf("❌ Error LLM: %v\n", err); return }

	msg := resp.Choices[0].Message
	if len(msg.ToolCalls) > 0 {
		tc := msg.ToolCalls[0]
		if tc.ID == "" { tc.ID = fmt.Sprintf("call_%d", time.Now().UnixNano()) }
		fmt.Printf("🔧 Herramienta: %s\n", tc.Function.Name)
		
		result := execTool(tc.Function.Name)
		fmt.Printf("⚙️ Resultado: %s\n", result)
		
		msgs = append(msgs, Message{Role: "assistant", ToolCalls: []ToolCall{tc}}, Message{Role: "tool", Content: result, ToolCallID: tc.ID})
		
		final, err := callLLM(msgs, nil)
		if err != nil { fmt.Printf("❌ Segunda llamada: %v\n", err); return }
		ans := final.Choices[0].Message.Content
		fmt.Printf("💬 Respuesta: %s\n", ans)
		saveMemory(userQuery, ans)
	} else {
		ans := msg.Content
		fmt.Printf("💬 Directa: %s\n", ans)
		saveMemory(userQuery, ans)
	}
}
