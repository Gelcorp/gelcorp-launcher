<script>
  import MainPage from "$/pages/MainPage.svelte";
  import LoginPage from "$/pages/LoginPage.svelte";
  import UpdatePage from "$/pages/UpdatePage.svelte";
  import AlertBoxLayout from "$/components/AlertBoxLayout.svelte";

  import { launcherConfigStore } from "$/ipc/stores/launcher_config";
  import { checkUpdate } from "@tauri-apps/api/updater";

  $: authenticated = $launcherConfigStore?.authentication !== undefined;

  let showUpdateScreen = true;
  let update = new Promise(async (resolve) => {
    try {
      let update = await checkUpdate();
      resolve(update);
    } catch (err) {
      console.error(`Error checking for updates: ${err}`);
      resolve({ shouldUpdate: false });
    }
  });
</script>

{#await update}
  <AlertBoxLayout>
    <h4>Buscando actualizaciones...</h4>
  </AlertBoxLayout>
{:then { manifest, shouldUpdate }}
  {#if showUpdateScreen && shouldUpdate}
    <UpdatePage on:close={() => (showUpdateScreen = false)} {manifest} />
  {:else if authenticated}
    <MainPage />
  {:else}
    <LoginPage />
  {/if}
{/await}
