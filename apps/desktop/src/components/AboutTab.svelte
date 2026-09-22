<script lang="ts">
  import { onMount } from "svelte";
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
  <h2>Reroute</h2>
  <p>Choose a browser for each link. Free software under the MIT license.</p>
  {#if info}
    <dl>
      <dt>Version</dt>
      <dd>{info.version}</dd>
      <dt>Platform</dt>
      <dd>{info.platform}</dd>
      <dt>Configuration file</dt>
      <dd><code>{info.config_path}</code></dd>
    </dl>
  {/if}

  <div class="row">
    <button type="button" onclick={checkVersion} disabled={checking}>
      {checking ? "Checking…" : "Check for a new version"}
    </button>
    {#if latest?.newer}
      <button type="button" class="primary" onclick={openReleasePage}>Open the download page</button>
    {/if}
  </div>

  <div role="status" aria-live="polite">
    {#if latest?.newer}
      Version <strong>{latest.version}</strong> is available.
    {:else if latest}
      You have the latest version ({latest.version}).
    {/if}
  </div>

  {#if error}
    <Banner tone="error">{error}</Banner>
  {/if}

  <p class="muted">
    Reroute forwards only <code>http</code> and <code>https</code> links, and only to executables
    listed in its configuration file. The button above is the one time it uses the network: it asks
    which version is newest and nothing else. No download, no installation, and nothing sent about
    you or your browsers.
  </p>
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
