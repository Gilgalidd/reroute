<script lang="ts">
  import Picker from "./routes/Picker.svelte";
  import Settings from "./routes/Settings.svelte";

  // The Rust side opens each window on `index.html#/picker` or
  // `index.html#/settings`; nothing else is routable.
  let route = $state(window.location.hash);
  $effect(() => {
    const update = () => (route = window.location.hash);
    window.addEventListener("hashchange", update);
    return () => window.removeEventListener("hashchange", update);
  });
</script>

{#if route === "#/settings"}
  <Settings />
{:else}
  <Picker />
{/if}
