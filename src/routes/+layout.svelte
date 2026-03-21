<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    document.title = "PaperDesk";
  });
</script>

{@render children()}
