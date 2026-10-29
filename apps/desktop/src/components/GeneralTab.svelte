<script lang="ts">
  import { onMount } from "svelte";
  import { LANGUAGES, languageName, t, tParts, tRust } from "../i18n/i18n.svelte";
  import { api, describeError } from "../lib/api";
  import { normalizeConfig } from "../lib/config";
  import type { Config, ImportReport } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable() }: { config: Config } = $props();

  let isDefault = $state<boolean | null>(null);
  let registerMessage = $state<{ tone: "info" | "error" | "success"; text: string } | null>(null);
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
          ? { tone: "success", text: t("general.default.done") }
          : { tone: "info", text: tRust(outcome.message) };
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

  function imported(report: ImportReport): string {
    return t("general.hurl.done", {
      browsers: t("general.hurl.browsers", { count: report.browsers_added }),
      rulesets: t("general.hurl.rulesets", { count: report.rulesets_added }),
    });
  }

  onMount(() => {
    void refreshStatus();
  });
</script>

<div class="general">
  <section class="card">
    <h3>{t("general.default")}</h3>
    {#if isDefault === true}
      <Banner tone="success">{t("general.default.yes")}</Banner>
    {:else if isDefault === false}
      <p>{t("general.default.no")}</p>
    {:else}
      <p class="muted">{t("general.default.unknown")}</p>
    {/if}
    <div class="row">
      <button type="button" class="primary" onclick={register}>{t("general.default.make")}</button>
      <button type="button" onclick={refreshStatus}>{t("general.default.recheck")}</button>
    </div>
    {#if registerMessage}
      <Banner tone={registerMessage.tone}>{registerMessage.text}</Banner>
    {/if}
  </section>

  <section class="card">
    <h3>{t("general.behaviour")}</h3>
    <label class="row"><input type="checkbox" bind:checked={config.settings.rules_enabled} /> {t("general.rulesEnabled")}</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.offer_remember} /> {t("general.offerRemember")}</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.close_on_focus_loss} /> {t("general.closeOnFocusLoss")}</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.open_under_cursor} /> {t("general.openUnderCursor")}</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.run_in_background} /> {t("general.runInBackground")}</label>
    <label class="row"><input type="checkbox" bind:checked={config.settings.check_for_updates} /> {t("general.checkForUpdates")}</label>
    <label class="row">
      <span>{t("general.language")}</span>
      <select bind:value={config.settings.language} class="narrow">
        <option value="auto">{t("general.language.auto")}</option>
        <!-- Each language in its own words, which no translation changes. -->
        {#each LANGUAGES as tag (tag)}
          <option value={tag} lang={tag} translate="no">{languageName(tag)}</option>
        {/each}
      </select>
    </label>
    <label class="row">
      <span>{t("general.theme")}</span>
      <select bind:value={config.settings.theme} class="narrow">
        <option value="auto">{t("general.theme.auto")}</option>
        <option value="light">{t("general.theme.light")}</option>
        <option value="dark">{t("general.theme.dark")}</option>
      </select>
    </label>
    <label class="row">
      <span>{t("general.layout")}</span>
      <select bind:value={config.settings.picker_layout} class="narrow">
        <option value="tiles">{t("general.layout.tiles")}</option>
        <option value="list">{t("general.layout.list")}</option>
        <option value="two-columns">{t("general.layout.twoColumns")}</option>
      </select>
    </label>
  </section>

  <section class="card">
    <h3>{t("general.hurl")}</h3>
    <!-- On one line: whitespace between the parts would show. -->
    <p class="muted">{#each tParts("general.hurl.help", { file: "UserSettings.json" }) as part}{#if part.slot === "file"}<code>{part.text}</code>{:else}{part.text}{/if}{/each}</p>
    <textarea rows="5" bind:value={hurlJson} placeholder={"{ \"Browsers\": [...], \"Rulesets\": [...] }"} spellcheck="false" translate="no"></textarea>
    <div class="row">
      <button type="button" onclick={importHurl} disabled={hurlJson.trim().length === 0}>{t("general.hurl.import")}</button>
    </div>
    {#if importError}
      <Banner tone="error">{importError}</Banner>
    {:else if importReport}
      <Banner tone="success">{imported(importReport)}</Banner>
      {#if importReport.notes.length}
        <ul class="notes">
          <!-- Not keyed by text: two notes can read the same. -->
          {#each importReport.notes as note}<li>{tRust(note)}</li>{/each}
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
