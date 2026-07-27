<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { stripPunctuation, tokenize } from "./tokenize";

  let { onClose }: { onClose: () => void } = $props();

  let frase = $state("");
  let heard = $state("");
  let correct = $state("");
  let error = $state("");
  let guardando = $state(false);

  let tokens = $derived(tokenize(frase));
  // No se puede guardar una corrección sin las dos puntas.
  let listo = $derived(heard !== "" && correct.trim() !== "" && !guardando);

  // Al abrir, se le pide al backend la última frase tal cual la escuchó
  // Whisper: es la única en la que aparece la palabra mal entendida.
  $effect(() => {
    invoke<string>("get_last_transcription")
      .then((texto) => (frase = texto))
      .catch((e) => {
        console.error("get_last_transcription falló:", e);
        error = "No se pudo leer la última transcripción.";
      });
  });

  async function guardar() {
    guardando = true;
    error = "";
    try {
      await invoke("add_vocab_word", { heard, correct: correct.trim() });
      onClose();
    } catch (e) {
      console.error("add_vocab_word falló:", e);
      error = "No se pudo guardar la corrección.";
      guardando = false;
    }
  }
</script>

<div class="tarjeta">
  <p class="titulo">¿Qué palabra entendió mal?</p>

  {#if tokens.length === 0}
    <p class="vacio">Todavía no dictaste nada.</p>
  {:else}
    <div class="chips">
      {#each tokens as token, i (i)}
        {@const palabra = stripPunctuation(token)}
        <button
          class="chip"
          class:elegido={heard === palabra}
          disabled={palabra === ""}
          onclick={() => (heard = palabra)}
        >
          {token}
        </button>
      {/each}
    </div>
  {/if}

  <label class="campo">
    <span>Se escribe</span>
    <input
      type="text"
      bind:value={correct}
      placeholder={heard ? `en vez de "${heard}"` : "elegí una palabra primero"}
      onkeydown={(e) => {
        if (e.key === "Enter" && listo) guardar();
        if (e.key === "Escape") onClose();
      }}
    />
  </label>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="acciones">
    <button class="secundario" onclick={onClose}>Cancelar</button>
    <button class="primario" disabled={!listo} onclick={guardar}>Guardar</button>
  </div>
</div>

<style>
  .tarjeta {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border-radius: 20px;
    border: 0.5px solid rgba(255, 255, 255, 0.25);
    background: rgba(245, 245, 247, 0.82);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.18);
    color: rgba(28, 28, 30, 0.9);
  }

  .titulo {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .vacio {
    margin: 0;
    font-size: 12px;
    opacity: 0.6;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-height: 96px;
    overflow-y: auto;
  }

  .chip {
    padding: 4px 10px;
    border: none;
    border-radius: 14px;
    background: rgba(120, 120, 128, 0.16);
    color: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .chip:hover:not(:disabled) {
    background: rgba(120, 120, 128, 0.28);
  }

  .chip.elegido {
    background: #0a84ff;
    color: #fff;
  }

  .chip:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .campo {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }

  .campo span {
    opacity: 0.7;
  }

  input {
    padding: 7px 10px;
    border: 0.5px solid rgba(60, 60, 67, 0.29);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.7);
    color: inherit;
    font: inherit;
    font-size: 13px;
  }

  input:focus {
    outline: 2px solid rgba(10, 132, 255, 0.9);
    outline-offset: -1px;
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: #d70015;
  }

  .acciones {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .acciones button {
    padding: 6px 14px;
    border: none;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .secundario {
    background: rgba(120, 120, 128, 0.16);
    color: inherit;
  }

  .primario {
    background: #0a84ff;
    color: #fff;
  }

  .primario:disabled {
    opacity: 0.4;
    cursor: default;
  }

  @media (prefers-color-scheme: dark) {
    .tarjeta {
      border-color: rgba(255, 255, 255, 0.12);
      background: rgba(44, 44, 46, 0.85);
      color: rgba(245, 245, 247, 0.9);
    }

    .chip {
      background: rgba(235, 235, 245, 0.16);
    }

    .chip:hover:not(:disabled) {
      background: rgba(235, 235, 245, 0.28);
    }

    input {
      border-color: rgba(235, 235, 245, 0.2);
      background: rgba(28, 28, 30, 0.6);
    }

    .secundario {
      background: rgba(235, 235, 245, 0.16);
    }

    .error {
      color: #ff453a;
    }
  }
</style>
