<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { isAnimated, status, statusLabel, type Status } from "./state";

  let { onAddWord }: { onAddWord: () => void } = $props();

  onMount(() => {
    // El backend manda el estado en cada cambio; la píldora nunca lo adivina.
    // El `catch` cubre correr fuera de Tauri (por ejemplo `npm run preview`,
    // que es como se revisa el diseño): ahí no hay backend que escuchar y la
    // píldora tiene que dibujarse igual, no quedar en una promesa rechazada.
    const stop = listen<Status>("state-changed", (event) => {
      status.set(event.payload);
    }).catch((error) => {
      console.error("no se pudo escuchar state-changed:", error);
      return () => {};
    });

    return () => {
      void stop.then((unlisten) => unlisten());
    };
  });

  async function toggle() {
    // Mientras `toggle_dictation` no exista (llega con commands.rs en la
    // Task 12), invoke rechaza. Se traga el error a propósito: una píldora que
    // explota en la cara del usuario es peor que una que todavía no hace nada.
    try {
      status.set(await invoke<Status>("toggle_dictation"));
    } catch (error) {
      console.error("toggle_dictation falló:", error);
    }
  }
</script>

<!--
  La ventana entera es la píldora: sin bordes, translúcida y arrastrable.
  `data-tauri-drag-region` es lo que la hace arrastrable desde cualquier parte
  del cuerpo, menos los controles.
-->
<div
  class="pill"
  class:activa={isAnimated($status)}
  data-tauri-drag-region
  role="button"
  tabindex="0"
  aria-label={statusLabel($status)}
  onclick={toggle}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      toggle();
    }
  }}
>
  <div class="ondas" aria-hidden="true">
    {#each [0, 1, 2, 3, 4] as barra (barra)}
      <span class="barra" style="--retraso: {barra * 110}ms"></span>
    {/each}
  </div>

  <span class="texto">{statusLabel($status)}</span>

  <button
    class="agregar"
    title="Corregir una palabra"
    aria-label="Corregir una palabra"
    onclick={(e) => {
      // Sin esto el click también togglearía el dictado.
      e.stopPropagation();
      onAddWord();
    }}
  >
    +
  </button>
</div>

<style>
  .pill {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 100%;
    padding: 0 10px 0 16px;
    box-sizing: border-box;

    border-radius: 28px;
    border: 0.5px solid rgba(255, 255, 255, 0.25);
    background: rgba(245, 245, 247, 0.68);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.18);

    cursor: default;
    transition: background 200ms ease;
  }

  .pill:focus-visible {
    outline: 2px solid rgba(10, 132, 255, 0.9);
    outline-offset: 2px;
  }

  .ondas {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 22px;
    flex: none;
  }

  .barra {
    width: 3px;
    height: 5px;
    border-radius: 2px;
    background: rgba(60, 60, 67, 0.55);
    transition: background 200ms ease;
  }

  /* En reposo las barras quedan quietas y bajas: la píldora no simula
     escuchar cuando no está escuchando. */
  .pill.activa .barra {
    background: #0a84ff;
    animation: onda 900ms ease-in-out infinite;
    animation-delay: var(--retraso);
  }

  @keyframes onda {
    0%,
    100% {
      height: 5px;
    }
    50% {
      height: 20px;
    }
  }

  .texto {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
    color: rgba(28, 28, 30, 0.85);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .agregar {
    flex: none;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: rgba(120, 120, 128, 0.16);
    color: rgba(28, 28, 30, 0.85);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .agregar:hover {
    background: rgba(120, 120, 128, 0.28);
  }

  @media (prefers-color-scheme: dark) {
    .pill {
      border-color: rgba(255, 255, 255, 0.12);
      background: rgba(44, 44, 46, 0.68);
      box-shadow: 0 8px 30px rgba(0, 0, 0, 0.45);
    }

    .barra {
      background: rgba(235, 235, 245, 0.45);
    }

    .texto,
    .agregar {
      color: rgba(245, 245, 247, 0.9);
    }

    .agregar {
      background: rgba(235, 235, 245, 0.16);
    }

    .agregar:hover {
      background: rgba(235, 235, 245, 0.28);
    }
  }

  /* Respeta a quien pidió menos movimiento en el sistema. */
  @media (prefers-reduced-motion: reduce) {
    .pill.activa .barra {
      animation: none;
      height: 14px;
    }
  }
</style>
