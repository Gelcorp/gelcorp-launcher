<script>
  import MainPage from "./MainPage.svelte";
  import LoginPage from "./LoginPage.svelte";
  import { launcherConfigStore } from "./stores/launcher_config";
</script>

{#if $launcherConfigStore.authentication}
  <MainPage />
{:else}
  <LoginPage />
{/if}
