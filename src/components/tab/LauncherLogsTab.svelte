<script lang="ts">
  import Logs from "$/components/Logs.svelte";
  import { launcherLogsStore } from "$/ipc/stores/loggers";
</script>

<Logs logs={$launcherLogsStore} />
