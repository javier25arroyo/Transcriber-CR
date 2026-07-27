<script lang="ts">
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import AddWordPopup from "./lib/AddWordPopup.svelte";
  import PillWidget from "./lib/PillWidget.svelte";

  // El popup se abre desde el "+" de la píldora y comparte la ventana con
  // ella. Es una sola ventana, no dos: así arrastrás la píldora y el popup la
  // sigue, y no quedan dos cosas separadas flotando sobre el escritorio.
  let mostrandoPopup = $state(false);

  // Alto de la ventana en cada estado. Ojo: la ventana es transparente y
  // siempre-encima, así que todo píxel de ventana que no se vea igual se come
  // los clicks de lo que haya abajo. Si quedara siempre alta, la persona no
  // podría hacer click en un pedazo de su propia pantalla. Por eso la ventana
  // crece sólo mientras el popup está abierto.
  const ALTO_PILDORA = 64;
  const ALTO_CON_POPUP = 260;
  const ANCHO = 300;

  async function mostrar(abierto: boolean) {
    mostrandoPopup = abierto;
    const alto = abierto ? ALTO_CON_POPUP : ALTO_PILDORA;
    try {
      await getCurrentWindow().setSize(new LogicalSize(ANCHO, alto));
    } catch (error) {
      // Fuera de Tauri (`npm run preview`, que es como se revisa el diseño) no
      // hay ventana que redimensionar. El popup se abre igual.
      console.error("no se pudo redimensionar la ventana:", error);
    }
  }
</script>

<main>
  <div class="fila-pildora">
    <PillWidget onAddWord={() => mostrar(true)} />
  </div>

  {#if mostrandoPopup}
    <div class="fila-popup">
      <AddWordPopup onClose={() => mostrar(false)} />
    </div>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100vh;
    padding: 4px;
    box-sizing: border-box;
    /* El fondo lo pone cada tarjeta: la ventana en sí es transparente. */
    background: transparent;
  }

  .fila-pildora {
    height: 56px;
    flex: none;
  }

  .fila-popup {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
</style>
