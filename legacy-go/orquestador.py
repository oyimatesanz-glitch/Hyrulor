import sys
from openai import OpenAI

client = OpenAI(base_url="http://localhost:11434/v1", api_key="ollama")

def pipeline(prompt):
    print("\n[1/2] DeepSeek analizando la lógica...")
    r1 = client.chat.completions.create(
        model="deepseek-r1:8b",
        messages=[{"role": "system", "content": "Eres un arquitecto de software experto. Diseña la solución paso a paso."},
                  {"role": "user", "content": prompt}]
    )
    razonamiento = r1.choices[0].message.content
    print("✔ Razonamiento listo.")

    print("\n[2/2] Qwen escribiendo el código...")
    r2 = client.chat.completions.create(
        model="qwen2.5-coder:7b",
        messages=[{"role": "system", "content": "Basándote en el razonamiento previo, escribe código limpio, seguro y listo para producción."},
                  {"role": "user", "content": f"Razonamiento:\n{razonamiento}\n\nImplementa el código."}]
    )
    print("\n" + "="*40 + "\n CÓDIGO FINAL:\n" + "="*40)
    print(r2.choices[0].message.content)

if __name__ == "__main__":
    prompt_usuario = " ".join(sys.argv[1:]) if len(sys.argv) > 1 else "Crea un script seguro en Rust."
    pipeline(prompt_usuario)
