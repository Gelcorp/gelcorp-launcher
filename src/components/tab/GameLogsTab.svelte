<script lang="ts">
  import Logs from "../Logs.svelte";
  import { gameLogsStore } from "$/ipc/stores/loggers";
</script>

<Logs logs={$gameLogsStore} />
