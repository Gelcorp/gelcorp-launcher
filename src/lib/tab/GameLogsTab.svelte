<script lang="ts">
  import Logs from "../Logs.svelte";
  import { gameLogsStore } from "../../stores/loggers";
</script>

<Logs logs={$gameLogsStore} />
