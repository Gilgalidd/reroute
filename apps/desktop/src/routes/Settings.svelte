<script lang="ts">
  import { onMount } from "svelte";
  import { type MessageKey, setLanguage, t, tRust } from "../i18n/i18n.svelte";
  import { api, describeError } from "../lib/api";
  import { applyTheme } from "../lib/theme";
  import { normalizeConfig } from "../lib/config";
  import type { Config, RustMessage } from "../lib/types";
  import Banner from "../components/Banner.svelte";
  import BrowsersTab from "../components/BrowsersTab.svelte";
  import RulesTab from "../components/RulesTab.svelte";
  import GeneralTab from "../components/GeneralTab.svelte";
  import AboutTab from "../components/AboutTab.svelte";

  type Tab = "browsers" | "rules" | "general" | "about";
  const tabs: { id: Tab; label: MessageKey }[] = [
    { id: "browsers", label: "settings.tab.browsers" },
    { id: "rules", label: "settings.tab.rules" },
    { id: "general", label: "settings.tab.general" },
    { id: "about", label: "settings.tab.about" },
  ];

  let tab = $state<Tab>("browsers");
  let draft = $state<Config | null>(null);
  let saved = $state("");
  let status = $state<{ tone: "info" | "error" | "success"; text: string } | null>(null);
  /** Why config.toml could not be read; the draft then holds only the installed browsers. */
  let loadError = $state<RustMessage | null>(null);

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
        ? { tone: "info", text: t("settings.savedBroken") }
        : { tone: "success", text: t("settings.saved") };
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
  // A language chosen in General shows at once, as the theme does; Discard
  // brings the saved one back.
  $effect(() => {
    if (draft) setLanguage(draft.settings.language);
  });
  // Rust names the window after the page's title.
  $effect(() => {
    document.title = t("settings.title");
  });
  $effect(() => {
    if (status?.tone === "success") {
      const timer = setTimeout(() => (status = null), 2500);
      return () => clearTimeout(timer);
    }
  });
</script>

<svelte:window
  onerror={(e) =>
    (status = {
      tone: "error",
      text: t("settings.unexpected", { detail: e instanceof ErrorEvent ? e.message : t("settings.unexpected.console") }),
    })}
  onunhandledrejection={(e) => (status = { tone: "error", text: t("settings.unexpected", { detail: describeError(e.reason) }) })}
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
  <nav aria-label={t("settings.sections")}>
    <div class="brand" translate="no">Reroute</div>
    {#each tabs as item (item.id)}
      <button type="button" class:active={tab === item.id} onclick={() => (tab = item.id)}>{t(item.label)}</button>
    {/each}
  </nav>

  <section class="content">
    {#if loadError}
      <Banner tone="error">{t("settings.loadError", { reason: tRust(loadError) })}</Banner>
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
      <p class="muted">{t("settings.loading")}</p>
    {/if}
  </section>

  <footer class="row">
    {#if status}
      <Banner tone={status.tone}>{status.text}</Banner>
    {:else if dirty}
      <span class="muted">{t("settings.unsaved")}</span>
    {/if}
    <span class="grow"></span>
    <button type="button" onclick={discard} disabled={!dirty}>{t("settings.discard")}</button>
    <button type="button" class="primary" onclick={save} disabled={!dirty}>{t("settings.save")}</button>
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
