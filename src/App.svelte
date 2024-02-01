<script>
  import MainPage from "$/pages/MainPage.svelte";
  import LoginPage from "$/pages/LoginPage.svelte";

  import { launcherConfigStore } from "$/ipc/stores/launcher_config";
  import { gameLogsStore } from "./ipc/stores/loggers";

  // Keep this active TODO: remove
  gameLogsStore.subscribe(() => {});

  const authenticated = $launcherConfigStore.authentication !== undefined;
</script>

{#if authenticated}
  <MainPage />
{:else}
  <LoginPage />
{/if}
