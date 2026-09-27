<script lang="ts">
  import { onMount } from "svelte";
  import { api, describeError } from "../lib/api";
  import { normalizeConfig } from "../lib/config";
  import type { Config, ImportReport } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable() }: { config: Config } = $props();

  let isDefault = $state<boolean | null>(null);
  let registerMessage = $state<{ tone: "info" | "error" | "success"; text: string } | null>(null);
  /** Staying in the background exists on Linux only. */
  let linux = $state(false);
  let hurlJson = $state("");
  let importReport = $state<ImportReport | null>(null);
  let importError = $state<string | null>(null);

  async function refreshStatus() {
    try {
      isDefault = await api.defaultBrowserStatus();
    } catch {
      isDefault = null;
    }
  }

  async function register() {
    registerMessage = null;
    try {
      const outcome = await api.registerDefaultBrowser();
      registerMessage =
        outcome.kind === "done"
          ? { tone: "success", text: "Reroute is now your default browser." }
          : { tone: "info", text: outcome.message };
      await refreshStatus();
    } catch (e) {
      registerMessage = { tone: "error", text: describeError(e) };
    }
  }

  /** Merge Hurl's settings into the draft; Save keeps them, like any edit. */
  async function importHurl() {
    importError = null;
    importReport = null;
    try {
      const report = await api.importHurl($state.snapshot(config), hurlJson);
      config = normalizeConfig(report.config);
      importReport = report;
      hurlJson = "";
    } catch (e) {
      importError = describeError(e);
    }
  }

  onMount(() => {
    void refreshStatus();
    api
      .appInfo()
      .then((info) => (linux = info.platform === "linux"))
      .catch(() => (linux = false));
  });
</script>

<div class="general">
  <section class="card">
    <h3>Default browser</h3>
    {#if isDefault === true}
      <Banner tone="success">Reroute is the default browser.</Banner>
    {:else if isDefault === false}
      <p>Reroute is not the default browser yet. Links will keep opening in your current browser.</p>
    {:else}
      <p class="muted">Could not determine the default browser on this system.</p>
    {/if}
    <div class="row">
      <button type="button" class="primary" onclick={register}>Make Reroute the default</button>
      <button type="button" onclick={refreshStatus}>Re-check</button>
    </div>
    {#if registerMessage}
      <Banner tone={registerMessage.tone}>{registerMessage.text}</Banner>
    {/if}
  </section>

  <section class="card">
    <h3>Behaviour</h3>
    <label class="row"><input type="checkbox" bind:checked={config.settings.rules_enabled} /> Let rules choose a browser without asking</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.offer_remember} /> Offer “always use for this domain” in the picker</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.close_on_focus_loss} /> Close the picker when it loses focus</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.open_under_cursor} /> Open the picker next to the mouse pointer</label>
    {#if linux}
      <label class="row"><input type="checkbox" bind:checked={config.settings.run_in_background} /> Keep Reroute running in the background so the picker opens at once (starts with your session)</label>
    {/if}
    <label class="row"><input type="checkbox" bind:checked={config.settings.check_for_updates} /> Check for a new version once a day, when Reroute starts</label>
    <label class="row">
      <span>Theme</span>
      <select bind:value={config.settings.theme} class="narrow">
        <option value="auto">Follow system</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
    <label class="row">
      <span>Show browsers as</span>
      <select bind:value={config.settings.picker_layout} class="narrow">
        <option value="tiles">Tiles</option>
        <option value="list">A list</option>
        <option value="two-columns">A list in two columns</option>
      </select>
    </label>
  </section>

  <section class="card">
    <h3>Import from Hurl</h3>
    <p class="muted">Paste the contents of Hurl's <code>UserSettings.json</code>. Its browsers and rules are added to the settings here; press Save to keep them.</p>
    <textarea rows="5" bind:value={hurlJson} placeholder={"{ \"Browsers\": [...], \"Rulesets\": [...] }"} spellcheck="false"></textarea>
    <div class="row">
      <button type="button" onclick={importHurl} disabled={hurlJson.trim().length === 0}>Import</button>
    </div>
    {#if importError}
      <Banner tone="error">{importError}</Banner>
    {:else if importReport}
      <Banner tone="success">Imported {importReport.browsers_added} browser(s) and {importReport.rulesets_added} ruleset(s). Press Save to keep them.</Banner>
      {#if importReport.notes.length}
        <ul class="notes">
          <!-- Not keyed by text: two notes can read the same. -->
          {#each importReport.notes as note}<li>{note}</li>{/each}
        </ul>
      {/if}
    {/if}
  </section>
</div>

<style>
  .general {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
  }
  .narrow {
    width: auto;
  }
  .notes {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--text-muted);
    user-select: text;
  }
</style>
