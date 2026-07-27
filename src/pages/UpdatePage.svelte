<script lang="ts">
  import AlertBoxLayout from "$/components/AlertBoxLayout.svelte";
  import ProgressBar from "$/components/ProgressBar.svelte";
  import { Update } from "@tauri-apps/plugin-updater";
  import { createEventDispatcher } from "svelte";

  export let update: Update;

  const dispatch = createEventDispatcher();

  let installing = false;
  let error: string | undefined;

  async function install() {
    try {
      installing = true;
      await update?.install();
    } catch (e) {
      console.error(e);
      error = String(e);
    } finally {
      installing = false;
    }
  }

  function close() {
    dispatch("close");
  }
</script>

<AlertBoxLayout>
  {#if error === undefined}
    <header>
      <img src="gelcorp-title.webp" alt="Logo de Gelcorp" />
      <h2>Actualización disponible</h2>
      <p>Es necesario actualizar a la versión <b>{update?.version}</b> para continuar</p>
    </header>
    <section>
      <b>Notas de actualización:</b>
      <code>
        {update?.body}
      </code>
    </section>
    <footer>
      <button on:click={install} disabled={installing}>Descargar e Instalar</button>
      {#if installing}
        <div class="progressbar-container">
          <ProgressBar progress={-1} --height="6px" />
        </div>
      {/if}
    </footer>
  {:else}
    <header>
      <img src="gelcorp-title.webp" alt="Logo de Gelcorp" />
    </header>
    <section class="error">
      <b>Error al descargar la actualización:</b>
      <code>{error}</code>
    </section>
    <footer>
      <button on:click={close}>Cerrar</button>
    </footer>
  {/if}
</AlertBoxLayout>

<style>
  header {
    text-align: center;
  }

  header img {
    height: 2.8rem;
  }

  section {
    text-align: left;
    overflow: hidden;
  }

  .error code {
    min-height: auto;
  }

  code {
    display: block;
    background-color: #313131;
    border-radius: 5px;
    min-height: 100px;
    padding: 2px 8px;
    text-align: left;
    text-wrap: wrap;

    overflow-y: auto;
    user-select: text;
  }

  footer {
    margin: 5px 0;
    text-align: center;
  }

  .progressbar-container {
    margin: 5px 0;
  }

  button {
    padding-block: 8px;
    width: 100%;

    margin-top: 5px;
    background-color: #e1e1e1;
    color: black;
    border: 1px solid gray;

    box-sizing: border-box;
  }
</style>
