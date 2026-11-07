package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"
)

func saveToMemory(userMsg, assistantMsg string) {
	memFile := "memory.json"

	// 1. Crear archivo si no existe o leerlo
	data, err := os.ReadFile(memFile)
	if err != nil || len(data) == 0 {
		// Inicializar estructura vacía si falla la lectura
		initData := map[string]interface{}{
			"conversations": []interface{}{},
		}
		data, _ = json.Marshal(initData)
	}

	// 2. Deserializar con validación
	var memory map[string]interface{}
	if err := json.Unmarshal(data, &memory); err != nil {
		fmt.Printf("⚠️ Error parseando memoria, reiniciando: %v\n", err)
		memory = map[string]interface{}{"conversations": []interface{}{}}
	}

	// 3. Asegurar que existe el array de conversaciones
	convs, ok := memory["conversations"].([]interface{})
	if !ok {
		convs = []interface{}{}
	}

	// 4. Añadir nueva entrada
	entry := map[string]interface{}{
		"timestamp": time.Now().Format(time.RFC3339),
		"user":      userMsg,
		"assistant": assistantMsg,
	}
	memory["conversations"] = append(convs, entry)

	// 5. Rotación: mantener solo últimas 100
	if len(memory["conversations"].([]interface{})) > 100 {
		memory["conversations"] = memory["conversations"].([]interface{})[len(memory["conversations"].([]interface{}))-100:]
	}

	// 6. Guardar a disco
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
