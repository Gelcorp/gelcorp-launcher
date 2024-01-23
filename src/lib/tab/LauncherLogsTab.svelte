<script lang="ts">
  import Logs from "./../Logs.svelte";
  import { launcherLogsStore } from "../../stores/loggers";
</script>

<Logs logs={$launcherLogsStore} />
