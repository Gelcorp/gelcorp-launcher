<script>
  import MainPage from "$/pages/MainPage.svelte";
  import LoginPage from "$/pages/LoginPage.svelte";

  import { launcherConfigStore } from "$/ipc/stores/launcher_config";

  $: authenticated = $launcherConfigStore?.authentication !== undefined;
</script>

{#if authenticated}
  <MainPage />
{:else}
  <LoginPage />
{/if}
