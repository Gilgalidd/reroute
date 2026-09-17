<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { AppInfo } from "../lib/types";

  let info = $state<AppInfo | null>(null);
  onMount(async () => {
    info = await api.appInfo();
  });
</script>

<div class="about">
  <h2>Signpost</h2>
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
  <p class="muted">
    Signpost never connects to the network. It only forwards <code>http</code> and <code>https</code> links, and only to
    executables listed in its configuration file.
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
</style>
