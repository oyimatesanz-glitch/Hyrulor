package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"time"
)

type Message struct {
	Role       string     `json:"role"`
	Content    string     `json:"content,omitempty"`
	ToolCalls  []ToolCall `json:"tool_calls,omitempty"`
	ToolCallID string     `json:"tool_call_id,omitempty"`
}

type ToolCall struct {
	ID       string         `json:"id"`
	Type     string         `json:"type"`
	Function ToolFunction   `json:"function"`
}

type ToolFunction struct {
	Name      string `json:"name"`
	Arguments string `json:"arguments"`
}

type ToolDef struct {
	Type     string             `json:"type"`
	Function FunctionDefinition `json:"function"`
}

type FunctionDefinition struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Parameters  map[string]interface{} `json:"parameters"`
}

type ChatRequest struct {
	Model       string    `json:"model"`
	Messages    []Message `json:"messages"`
	Tools       []ToolDef `json:"tools,omitempty"`
	Temperature float64   `json:"temperature"`
}

type ChatResponse struct {
	Choices []struct {
		Message Message `json:"message"`
	} `json:"choices"`
}

func saveToMemory(userMsg, assistantMsg string) {
	memFile := "memory.json"
	data, err := os.ReadFile(memFile)
	if err != nil || len(data) == 0 {
		initData := map[string]interface{}{"conversations": []interface{}{}}
		data, _ = json.Marshal(initData)
	}

	var memory map[string]interface{}
	if err := json.Unmarshal(data, &memory); err != nil {
		fmt.Printf("⚠️ Error parseando memoria, reiniciando: %v\n", err)
		memory = map[string]interface{}{"conversations": []interface{}{}}
	}

	convs, ok := memory["conversations"].([]interface{})
	if !ok {
		convs = []interface{}{}
	}

	entry := map[string]interface{}{
		"timestamp": time.Now().Format(time.RFC3339),
		"user":      userMsg,
		"assistant": assistantMsg,
	}
	memory["conversations"] = append(convs, entry)

	if len(memory["conversations"].([]interface{})) > 100 {
		memory["conversations"] = memory["conversations"].([]interface{})[len(memory["conversations"].([]interface{}))-100:]
	}

	updated, err := json.MarshalIndent(memory, "", "  ")
	if err != nil {
		fmt.Printf("❌ Error serializando memoria: %v\n", err)
		return
	}

	if err := os.WriteFile(memFile, updated, 0644); err != nil {
		fmt.Printf("❌ Error escribiendo memoria: %v\n", err)
	} else {
		fmt.Println("💾 Memoria guardada correctamente")
	}
}

func callLLM(messages []Message, tools []ToolDef) ChatResponse {
	reqBody, _ := json.Marshal(ChatRequest{
		Model:       "Qwen2.5-3B-Instruct-Q4_K_M",
		Messages:    messages,
		Tools:       tools,
		Temperature: 0.1,
	})

	req, err := http.NewRequest("POST", "http://localhost:8080/v1/chat/completions", bytes.NewBuffer(reqBody))
	if err != nil {
		fmt.Printf("❌ Error creando petición: %v\n", err)
		return ChatResponse{}
	}
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{}
	res, err := client.Do(req)
	if err != nil {
		fmt.Printf("❌ Error conectando al servidor: %v\n", err)
		return ChatResponse{}
	}
	defer res.Body.Close()

	body, _ := io.ReadAll(res.Body)
	var resp ChatResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		fmt.Printf("❌ Error parseando JSON: %v | Body: %s\n", err, string(body))
	}
	return resp
}

func main() {
	fmt.Println("🤖 Asistente Local Iniciado...")

	tools := []ToolDef{
		{
			Type: "function",
			Function: FunctionDefinition{
				Name:        "battery_status",
				Description: "Obtiene el estado actual y el porcentaje de la batería del sistema.",
				Parameters:  map[string]interface{}{"type": "object", "properties": map[string]interface{}{}, "required": []string{}},
			},
		},
	}

	messages := []Message{
		{Role: "system", Content: "Eres un asistente útil. Usa las herramientas cuando sea necesario."},
		{Role: "user", Content: "¿Cómo está la batería de mi laptop?"},
	}

	resp := callLLM(messages, tools)

	if len(resp.Choices) == 0 {
		fmt.Println("❌ No hubo respuesta del modelo.")
		return
	}

	msg := resp.Choices[0].Message

	if len(msg.ToolCalls) > 0 {
		tc := msg.ToolCalls[0]
		fmt.Printf("🔧 Modelo solicita herramienta: %s\n", tc.Function.Name)

		var result string
		switch tc.Function.Name {
		case "battery_status":
			out, _ := exec.Command("./battery_tool.sh").Output()
			result = strings.TrimSpace(string(out))
		default:
			result = `{"error": "Herramienta desconocida"}`
		}

		fmt.Printf("⚙️ Herramienta respondió: %s\n", result)

		messages = append(messages,
			Message{Role: "assistant", ToolCalls: []ToolCall{tc}},
			Message{Role: "tool", Content: result, ToolCallID: tc.ID},
		)

		finalResp := callLLM(messages, nil)
		finalContent := finalResp.Choices[0].Message.Content
		fmt.Printf("💬 Respuesta Final: %s\n", finalContent)
		
		// INTEGRACIÓN DE MEMORIA AQUÍ
		saveToMemory(msg.Content, finalContent)
	} else {
		directContent := msg.Content
		fmt.Printf("💬 Respuesta Directa: %s\n", directContent)
		
		// INTEGRACIÓN DE MEMORIA PARA RESPUESTAS DIRECTAS
		saveToMemory(messages[len(messages)-1].Content, directContent)
	}
}
