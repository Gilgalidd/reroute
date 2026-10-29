<script lang="ts">
  import { onMount } from "svelte";
  import { t, tParts } from "../i18n/i18n.svelte";
  import { api, describeError } from "../lib/api";
  import type { AppInfo, LatestRelease } from "../lib/types";
  import Banner from "./Banner.svelte";

  let info = $state<AppInfo | null>(null);
  let checking = $state(false);
  let latest = $state<LatestRelease | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    info = await api.appInfo();
  });

  async function checkVersion() {
    checking = true;
    error = null;
    latest = null;
    try {
      latest = await api.checkLatestRelease();
    } catch (e) {
      error = describeError(e);
    } finally {
      checking = false;
    }
  }

  async function openReleasePage() {
    error = null;
    try {
      await api.openReleasePage();
    } catch (e) {
      error = describeError(e);
    }
  }
</script>

<div class="about">
  <h2 translate="no">Reroute</h2>
  <p>{t("about.tagline")}</p>
  {#if info}
    <dl>
      <dt>{t("about.version")}</dt>
      <dd>{info.version}</dd>
      <dt>{t("about.platform")}</dt>
      <dd>{info.platform}</dd>
      <dt>{t("about.config")}</dt>
      <dd><code>{info.config_path}</code></dd>
    </dl>
  {/if}

  <div class="row">
    <button type="button" onclick={checkVersion} disabled={checking}>
      {checking ? t("about.checking") : t("about.check")}
    </button>
    {#if latest?.newer}
      <button type="button" class="primary" onclick={openReleasePage}>{t("about.download")}</button>
    {/if}
  </div>

  <!-- The parts on one line: whitespace between them would show. -->
  <div role="status" aria-live="polite">
    {#if latest?.newer}
      {#each tParts("about.available", { version: latest.version }) as part}{#if part.slot === "version"}<strong>{part.text}</strong>{:else}{part.text}{/if}{/each}
    {:else if latest}
      {t("about.latest", { version: latest.version })}
    {/if}
  </div>

  {#if error}
    <Banner tone="error">{error}</Banner>
  {/if}

  <p class="muted">{#each tParts("about.privacy", { http: "http", https: "https" }) as part}{#if part.slot}<code>{part.text}</code>{:else}{part.text}{/if}{/each}</p>
</div>

<style>
  .about {
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    margin: 0;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0;
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    user-select: text;
  }
  [role="status"]:empty {
    display: none;
  }
</style>
