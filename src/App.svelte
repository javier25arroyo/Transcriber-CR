<script lang="ts">
  import { status } from "./lib/state";
</script>

<!--
  Scaffold. La píldora flotante con la estética real (translúcida, redondeada,
  onda animada) llega en la Task 13 del plan de implementación; por ahora esta
  vista sólo confirma que el frontend monta y que el store de estado funciona.
-->
<main>
  <h1>Voice Type</h1>
  <p class="status">Estado: {$status}</p>
</main>

<style>
  main {
    display: grid;
    place-content: center;
    gap: 0.5rem;
    height: 100vh;
    text-align: center;
  }

  h1 {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
  }

  .status {
    margin: 0;
    opacity: 0.7;
  }
</style>
