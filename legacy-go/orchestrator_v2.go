package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os/exec"
	"strings"
	"time"
)

// ... (mismas structs Message, ToolCall, etc. que antes) ...
// [Para brevedad, asume que mantienes las structs anteriores]

func callLLM(messages []Message, tools []ToolDef) (ChatResponse, error) {
	body, _ := json.Marshal(ChatRequest{
		Model:       "Qwen2.5-3B-Instruct-Q4_K_M", // O SmolLM3 según tu prueba
		Messages:    messages,
		Tools:       tools,
		Temperature: 0.1,
	})

	req, _ := http.NewRequest("POST", "http://localhost:8080/v1/chat/completions", bytes.NewBuffer(body))
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 120 * time.Second}
	res, err := client.Do(req)
	if err != nil {
		return ChatResponse{}, fmt.Errorf("conexión fallida: %w", err)
	}
	defer res.Body.Close()

	var resp ChatResponse
	if err := json.NewDecoder(res.Body).Decode(&resp); err != nil {
		return ChatResponse{}, fmt.Errorf("JSON inválido: %w", err)
	}
	if len(resp.Choices) == 0 {
		return ChatResponse{}, fmt.Errorf("respuesta sin choices")
	}
	return resp, nil
}

func executeTool(name string) string {
	scripts := map[string]string{
		"battery_status": "./battery_tool.sh",
		"gpu_diag":       "./tools/gpu_diag.sh",
		"net_privacy":    "./tools/net_privacy.sh",
	}
	path, ok := scripts[name]
	if !ok {
		return fmt.Sprintf(`{"error":"herramienta desconocida: %s"}`, name)
	}
	out, err := exec.Command(path).Output()
	if err != nil {
		return fmt.Sprintf(`{"error":"ejecución fallida: %v"}`, err)
	}
	return strings.TrimSpace(string(out))
}

func main() {
	fmt.Println("🤖 Asistente V2 Seguro Iniciado...")
	
	// ... (definición de tools y messages igual que antes) ...
	
	resp, err := callLLM(messages, tools)
	if err != nil {
		fmt.Printf("❌ Error primera llamada: %v\n", err)
		return
	}
	
	msg := resp.Choices[0].Message
	if len(msg.ToolCalls) > 0 {
		tc := msg.ToolCalls[0]
		if tc.ID == "" {
			tc.ID = fmt.Sprintf("call_%d", time.Now().UnixNano())
		}
		
		fmt.Printf("🔧 Herramienta: %s\n", tc.Function.Name)
		result := executeTool(tc.Function.Name)
		fmt.Printf("⚙️ Resultado: %s\n", result)
		
		messages = append(messages,
			Message{Role: "assistant", ToolCalls: []ToolCall{tc}},
			Message{Role: "tool", Content: result, ToolCallID: tc.ID},
		)
		
		finalResp, err := callLLM(messages, nil)
		if err != nil {
			fmt.Printf("❌ Segunda llamada: %v\n", err)
			return
		}
		fmt.Printf("💬 Respuesta: %s\n", finalResp.Choices[0].Message.Content)
	} else {
		fmt.Printf("💬 Respuesta directa: %s\n", msg.Content)
	}
}
