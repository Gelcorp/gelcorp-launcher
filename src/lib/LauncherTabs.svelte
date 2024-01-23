<script lang="ts">
  import GameLogsTab from "./tab/GameLogsTab.svelte";
  import LauncherLogsTab from "./tab/LauncherLogsTab.svelte";
  import UpdateNotesTab from "./tab/UpdateNotesTab.svelte";
  import { launcherConfigStore } from "../stores/launcher_config";

  let username = $launcherConfigStore.authentication?.username;
  let tabs = [
    { label: "Notas de Actualización", tab: UpdateNotesTab },
    { label: "Logs del Launcher", tab: LauncherLogsTab },
    { label: `Logs del Juego (${username})`, tab: GameLogsTab },
  ];
  let selected = 0;

  function select(index: number) {
    selected = index % tabs.length;
  }
</script>

<div class="tabs">
  <nav>
    {#each tabs as tab, i}
      <button class:selected={i === selected} on:click={() => select(i)}>{tab.label}</button>
    {/each}
  </nav>
  <main>
    <div class="tab">
      <svelte:component this={tabs[selected].tab} />
    </div>
  </main>
</div>

<style>
  .tabs {
    background-color: rgb(158, 158, 158);
    display: grid;
    grid-template-rows: min-content 1fr;
    padding: 2px;
    padding-top: 0;

    height: 100%;
    overflow: hidden;
  }

  .tabs nav {
    display: flex;
    background-color: rgb(158, 158, 158);
    align-items: flex-end;
  }

  .tabs nav button {
    background-color: #e0e0e0;
    padding: 1px 6px;
    border: 1px solid gray;
    color: black;
    border-bottom: 0;
    outline: none;
  }

  .tabs nav button.selected {
    background-color: white;
    padding-top: 3px;
    padding-bottom: 2px;
    padding-right: 7px;
    position: relative;
    top: 1px;
    margin-right: -1px;
  }

  .tabs main {
    display: grid;
    background-color: white;
    border: 1px solid gray;
    padding: 2px;
    padding-bottom: 4px;

    overflow: hidden;
  }

  .tabs main .tab {
    border: 1px solid gray;
    color: black;
    height: 100%;
    overflow: auto;
  }
</style>
