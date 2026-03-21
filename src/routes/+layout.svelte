<script lang="ts">
  import "@fontsource/ibm-plex-sans/400.css";
  import "@fontsource/ibm-plex-sans/500.css";
  import "@fontsource/ibm-plex-sans/600.css";
  import "@fontsource/ibm-plex-sans/700.css";
  import "@fontsource/ibm-plex-mono/400.css";
  import "@fontsource/ibm-plex-mono/500.css";
  import "../app.css";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import { closeSettingsModal, settingsModal } from "$lib/settingsModal.svelte";
  import { locale, t } from "$lib/i18n/locale.svelte";

  let { children } = $props();

  $effect(() => {
    void locale.value;
    document.title = t("app.title");
  });
</script>

<div class="app-shell">
  {@render children()}
</div>

<SettingsModal
  open={settingsModal.open}
  onClose={closeSettingsModal}
/>

<style>
  .app-shell {
    position: relative;
    min-height: 100vh;
  }
</style>
