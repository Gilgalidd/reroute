<script lang="ts">
  import { onMount } from "svelte";
  import { api, describeError } from "../lib/api";
  import { applyTheme } from "../lib/theme";
  import { normalizeConfig } from "../lib/config";
  import type { Config } from "../lib/types";
  import Banner from "../components/Banner.svelte";
  import BrowsersTab from "../components/BrowsersTab.svelte";
  import RulesTab from "../components/RulesTab.svelte";
  import GeneralTab from "../components/GeneralTab.svelte";
  import AboutTab from "../components/AboutTab.svelte";

  type Tab = "browsers" | "rules" | "general" | "about";
  const tabs: { id: Tab; label: string }[] = [
    { id: "browsers", label: "Browsers" },
    { id: "rules", label: "Rules" },
    { id: "general", label: "General" },
    { id: "about", label: "About" },
  ];

  let tab = $state<Tab>("browsers");
  let draft = $state<Config | null>(null);
  let saved = $state("");
  let status = $state<{ tone: "info" | "error" | "success"; text: string } | null>(null);
  /** Why config.toml could not be read; the draft is then empty. */
  let loadError = $state<string | null>(null);

  const dirty = $derived(draft !== null && JSON.stringify(draft) !== saved);

  function adopt(config: Config) {
    const full = normalizeConfig(config);
    draft = full;
    saved = JSON.stringify(full);
  }

  async function load() {
    try {
      const editable = await api.getConfig();
      adopt(editable.config);
      loadError = editable.load_error;
    } catch (e) {
      status = { tone: "error", text: describeError(e) };
    }
  }

  async function save() {
    if (!draft) return;
    try {
      const snapshot = JSON.parse(JSON.stringify(draft)) as Config;
      await api.saveConfig(snapshot);
      adopt(snapshot);
      status = loadError
        ? { tone: "info", text: "Saved. The unreadable file is kept as config.toml.broken." }
        : { tone: "success", text: "Saved." };
      loadError = null;
    } catch (e) {
      status = { tone: "error", text: describeError(e) };
    }
  }

  function discard() {
    if (saved) draft = JSON.parse(saved) as Config;
    status = null;
  }

  onMount(() => void load());
  $effect(() => {
    if (draft) applyTheme(draft.settings.theme);
  });
  $effect(() => {
    if (status?.tone === "success") {
      const t = setTimeout(() => (status = null), 2500);
      return () => clearTimeout(t);
    }
  });
</script>

<svelte:window
  onerror={(e) => (status = { tone: "error", text: `Unexpected error: ${e instanceof ErrorEvent ? e.message : "see the console"}` })}
  onunhandledrejection={(e) => (status = { tone: "error", text: `Unexpected error: ${describeError(e.reason)}` })}
  onkeydown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      void save();
    }
  }}
  onbeforeunload={(e) => {
    if (dirty) e.preventDefault();
  }}
/>

<div class="layout">
  <nav aria-label="Settings sections">
    <div class="brand">Reroute</div>
    {#each tabs as t (t.id)}
      <button type="button" class:active={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
    {/each}
  </nav>

  <section class="content">
    {#if loadError}
      <Banner tone="error">
        config.toml could not be read: {loadError}. You are editing an empty configuration. Saving keeps the
        unreadable file as config.toml.broken, next to it, so you can repair it or copy rules from it.
      </Banner>
    {/if}
    {#if draft}
      {#if tab === "browsers"}
        <BrowsersTab bind:config={draft} />
      {:else if tab === "rules"}
        <RulesTab bind:config={draft} />
      {:else if tab === "general"}
        <GeneralTab bind:config={draft} />
      {:else}
        <AboutTab />
      {/if}
    {:else if status}
      <Banner tone={status.tone}>{status.text}</Banner>
    {:else}
      <p class="muted">Loading…</p>
    {/if}
  </section>

  <footer class="row">
    {#if status}
      <Banner tone={status.tone}>{status.text}</Banner>
    {:else if dirty}
      <span class="muted">Unsaved changes</span>
    {/if}
    <span class="grow"></span>
    <button type="button" onclick={discard} disabled={!dirty}>Discard</button>
    <button type="button" class="primary" onclick={save} disabled={!dirty}>Save</button>
  </footer>
</div>

<style>
  .layout {
    height: 100%;
    display: grid;
    grid-template-columns: 170px 1fr;
    grid-template-rows: 1fr auto;
  }
  nav {
    grid-row: 1 / 3;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 10px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }
  .brand {
    font-weight: 700;
    font-size: 16px;
    padding: 4px 10px 12px;
  }
  nav button {
    text-align: left;
    border: none;
    background: transparent;
  }
  nav button.active {
    background: var(--accent);
    color: var(--accent-text);
  }
  .content {
    padding: 18px 22px;
    overflow: auto;
  }
  footer {
    padding: 10px 22px;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
</style>
