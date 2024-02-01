<script lang="ts">
  import { totalMemoryStore } from "$/ipc/stores/system_info";
  import { launcherConfigStore } from "$/ipc/stores/launcher_config";
  import { modpackInfoStore, type Optional } from "$/ipc/stores/modpack_info";
  import { gameStatusStore, GameStatus } from "$/ipc/stores/game_status";
  import RamSlider from "../RamSlider.svelte";

  $: gameRunning = $gameStatusStore !== GameStatus.Idle;

  $: optionals = $modpackInfoStore?.optionals ?? [];
  let incompatibilities: { [key: string]: Optional[] } = {};
  $: selectedOptions = $launcherConfigStore.selected_options ?? [];

  $: {
    function addIncompatibility(optional_id: string, incompatibility: Optional) {
      let arr = incompatibilities[optional_id] ?? [];
      incompatibilities[optional_id] = [...arr, incompatibility];
    }

    incompatibilities = {};
    for (const optional of optionals) {
      if (optional.incompatible_with === undefined) continue;
      let mappedIncompatibilities = optionals.filter(({ id }) => optional.incompatible_with?.includes(id));
      mappedIncompatibilities.forEach((inc) => {
        addIncompatibility(optional.id, inc);
        addIncompatibility(inc.id, optional);
      });
    }
  }

  $: toggleSelect = (id: string) => {
    launcherConfigStore.update((config) => {
      if (selectedOptions.includes(id)) {
        config.selected_options = selectedOptions.filter((opt) => opt !== id);
      } else {
        config.selected_options = [...selectedOptions, id];
      }
      return config;
    });
  };

  $: getIncompatibilities = (id: string) => {
    return incompatibilities[id] ?? [];
  };

  $: getEnabledIncompatibilities = (id: string) => {
    return getIncompatibilities(id)
      .filter((inc) => selectedOptions.find((id) => id === inc.id))
      .map((inc) => inc.name);
  };

  let memoryElement: HTMLInputElement;
  launcherConfigStore.subscribe((config) => {
    if (memoryElement) memoryElement.value = String(config.memory_max);
  });

  // Rounded gb
  $: totalMem = Math.ceil($totalMemoryStore / 1024 / 1024 / 1024) * 1024;
</script>

<main>
  <h2>Configuración del juego:</h2>
  <section class="category">
    {#if totalMem > 0}
      <label for="memory">
        Memoria RAM:
        <RamSlider bind:value={$launcherConfigStore.memory_max} min={512} max={totalMem} disabled={gameRunning} />
      </label>
    {/if}
  </section>
  <h2>Mods Opcionales:</h2>
  <section class="opt-container">
    {#each optionals as { description, icon, id, name }}
      {@const selected = selectedOptions.includes(id)}
      {@const allIncompatibilities = getIncompatibilities(id)}
      {@const enabledIncompatibilities = getEnabledIncompatibilities(id)}

      <article class="opt-card">
        <img src={icon} alt="" />
        <section>
          <div class="top">
            <h3>{name}</h3>
            <p>{description}</p>
            {#if allIncompatibilities.length > 0}
              <p>
                No es compatible con: <b>{allIncompatibilities.map((inc) => inc.name).join(", ")}</b>
              </p>
            {/if}
          </div>
          <div class="bottom">
            <button
              class:red={selected}
              disabled={enabledIncompatibilities.length > 0 || gameRunning}
              on:click={() => enabledIncompatibilities.length == 0 && toggleSelect(id)}>{selected ? "Desactivar" : "Activar"}</button
            >
            {#if enabledIncompatibilities.length > 0}
              <span class="red">Incompatible con <b>{enabledIncompatibilities.join(", ")}</b></span>
            {/if}
          </div>
        </section>
      </article>
    {/each}
  </section>
</main>

<style>
  * {
    margin: 0;
    padding: 0;
    font-family: sans-serif;
  }

  main {
    color: white;
    background-color: #151515;
    height: 100%;
    padding: 20px;
    box-sizing: border-box;
  }

  h2 {
    margin-bottom: 20px;
  }

  .opt-container {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(700px, 1fr));
    margin: 5px 0;
    gap: 5px;
  }

  .opt-card {
    display: flex;
    min-width: 500px;

    image-rendering: pixelated;
    gap: 10px;
    border-radius: 10px;
  }

  .opt-card img {
    height: 100px;
    aspect-ratio: 1 / 1;
    object-fit: cover;
  }

  .opt-card section {
    box-sizing: border-box;
    padding-block: 10px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 100%;
  }

  .opt-card button {
    padding: 5px 18px;
    background-color: #5cb144;
    color: #fff;
    border: 0;
  }

  .opt-card button.red {
    background-color: #f00;
    color: #fff;
  }

  .opt-card button:disabled {
    background-color: #979797 !important;
  }

  .opt-card button:hover {
    filter: brightness(0.95) !important;
  }

  .opt-card span.red {
    color: rgb(255, 45, 45);
  }
</style>
