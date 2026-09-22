<script lang="ts">
  import { onMount } from "svelte";
  import { api, describeError } from "../lib/api";
  import type { Config, ImportReport, LatestRelease } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable() }: { config: Config } = $props();

  let isDefault = $state<boolean | null>(null);
  let registerMessage = $state<{ tone: "info" | "error" | "success"; text: string } | null>(null);
  let checking = $state(false);
  let latest = $state<LatestRelease | null>(null);
  let versionError = $state<string | null>(null);

  async function checkVersion() {
    checking = true;
    versionError = null;
    latest = null;
    try {
      latest = await api.checkLatestRelease();
    } catch (e) {
      versionError = describeError(e);
    } finally {
      checking = false;
    }
  }

  async function openReleasePage() {
    versionError = null;
    try {
      await api.openReleasePage();
    } catch (e) {
      versionError = describeError(e);
    }
  }

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

  async function importHurl() {
    importError = null;
    importReport = null;
    try {
      importReport = await api.importHurl(hurlJson);
      hurlJson = "";
    } catch (e) {
      importError = describeError(e);
    }
  }

  onMount(() => void refreshStatus());
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
    <label class="row">
      <span>Theme</span>
      <select bind:value={config.settings.theme} class="narrow">
        <option value="auto">Follow system</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
  </section>

  <section class="card">
    <h3>Version</h3>
    <div class="row">
      <button type="button" onclick={checkVersion} disabled={checking}>
        {checking ? "Checking…" : "Check for a new version"}
      </button>
      {#if latest?.newer}
        <button type="button" class="primary" onclick={openReleasePage}>
          Open the download page
        </button>
      {/if}
    </div>
    <div role="status" aria-live="polite">
      {#if latest?.newer}
        Version <strong>{latest.version}</strong> is available.
      {:else if latest}
        You have the latest version ({latest.version}).
      {/if}
    </div>
    {#if versionError}
      <Banner tone="error">{versionError}</Banner>
    {/if}
    <p class="muted">
      Checking asks GitHub which version is newest, and nothing else: no download, no
      installation, and nothing sent about you or your browsers. It happens only when you press
      the button.
    </p>
  </section>

  <section class="card">
    <h3>Import from Hurl</h3>
    <p class="muted">Paste the contents of Hurl's <code>UserSettings.json</code>. Browsers and rules are merged into your configuration and saved immediately.</p>
    <textarea rows="5" bind:value={hurlJson} placeholder={"{ \"Browsers\": [...], \"Rulesets\": [...] }"} spellcheck="false"></textarea>
    <div class="row">
      <button type="button" onclick={importHurl} disabled={hurlJson.trim().length === 0}>Import</button>
      <span class="muted">Reload this window afterwards to see the merged configuration.</span>
    </div>
    {#if importError}
      <Banner tone="error">{importError}</Banner>
    {:else if importReport}
      <Banner tone="success">Imported {importReport.browsers_added} browser(s) and {importReport.rulesets_added} ruleset(s).</Banner>
      {#if importReport.notes.length}
        <ul class="notes">
          {#each importReport.notes as note (note)}<li>{note}</li>{/each}
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
