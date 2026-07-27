<script>
  import MainPage from "$/pages/MainPage.svelte";
  import LoginPage from "$/pages/LoginPage.svelte";
  import UpdatePage from "$/pages/UpdatePage.svelte";
  import AlertBoxLayout from "$/components/AlertBoxLayout.svelte";

  import { launcherConfigStore } from "$/ipc/stores/launcher_config";
  import { check } from "@tauri-apps/plugin-updater";

  $: authenticated = $launcherConfigStore?.authentication !== undefined;

  let showUpdateScreen = true;
  let update = new Promise(async (resolve) => {
    try {
      let update = await check();
      resolve(update);
    } catch (err) {
      console.error(`Error checking for updates: ${err}`);
      resolve(null);
    }
  });
</script>

{#await update}
  <AlertBoxLayout>
    <h4>Buscando actualizaciones...</h4>
  </AlertBoxLayout>
{:then update}
  {#if showUpdateScreen && update !== null}
    <UpdatePage on:close={() => (showUpdateScreen = false)} {update} />
  {:else if authenticated}
    <MainPage />
  {:else}
    <LoginPage />
  {/if}
{/await}
