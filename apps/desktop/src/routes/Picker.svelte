<script lang="ts">
  import { onMount } from "svelte";
  import { api, describeError } from "../lib/api";
  import { mapKey, moveIndex } from "../lib/keys";
  import { applyTheme } from "../lib/theme";
  import type { BrowserView, LaunchContext } from "../lib/types";
  import Banner from "../components/Banner.svelte";
  import BrowserTile from "../components/BrowserTile.svelte";
  import LaunchMenu from "../components/LaunchMenu.svelte";

  let context = $state<LaunchContext | null>(null);
  let error = $state<string | null>(null);
  let highlighted = $state(0);
  let remember = $state(false);
  let busy = $state(false);
  let menu = $state<{ browser: BrowserView; anchor: HTMLElement } | null>(null);

  /** Tiles per row, mirroring `grid-template-columns` below: arrow keys move
   * by this much and the picker window is not resizable. */
  const columns = 4;

  const browsers = $derived(context?.browsers ?? []);
  const canPick = $derived(!!context?.url && browsers.length > 0);

  async function load() {
    try {
      context = await api.launchContext();
      applyTheme(context.settings.theme);
      highlighted = 0;
      remember = false;
      menu = null;
    } catch (e) {
      error = describeError(e);
    }
  }

  onMount(() => {
    void load();
    const unlisten = api.onContextChanged(() => void load());
    return () => {
      void unlisten.then((f) => f());
    };
  });

  async function pick(browser: BrowserView, launch: string | null) {
    if (busy || !context?.url) return;
    busy = true;
    error = null;
    try {
      await api.pick(browser.id, launch, remember);
    } catch (e) {
      error = describeError(e);
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (menu) return;
    const action = mapKey(e.key, columns);
    if (action.kind === "none") return;
    e.preventDefault();
    switch (action.kind) {
      case "select": {
        const b = browsers[action.index];
        if (b) void pick(b, null);
        break;
      }
      case "move":
        highlighted = moveIndex(highlighted, action.delta, browsers.length);
        break;
      case "confirm": {
        const b = browsers[highlighted];
        if (b) void pick(b, null);
        break;
      }
      case "menu": {
        const b = browsers[highlighted];
        const anchor = document.activeElement;
        if (b && b.launches.length && anchor instanceof HTMLElement) menu = { browser: b, anchor };
        break;
      }
      case "cancel":
        void api.dismiss();
        break;
      case "settings":
        void api.openSettings();
        break;
    }
  }
</script>

<svelte:window onkeydown={onkeydown} />

<main class="picker">
  <div class="strip" aria-hidden="true"></div>

  <header>
    {#if context?.url}
      <div class="host" title={context.url.href}>{context.url.host}</div>
      <div class="href muted" title={context.url.href}>{context.url.href}</div>
    {:else if context?.url_error}
      <Banner tone="error">This link was refused: {context.url_error}</Banner>
    {:else}
      <div class="host">Reroute</div>
      <div class="href muted">No link to open.</div>
    {/if}
  </header>

  {#if context?.config_error}
    <Banner tone="error">Configuration not loaded: {context.config_error}</Banner>
  {/if}
  {#if error}
    <Banner tone="error">{error}</Banner>
  {/if}

  {#if context && browsers.length === 0}
    <div class="empty">
      <p>No browsers configured yet.</p>
      <button class="primary" type="button" onclick={() => api.openSettings()}>Open settings</button>
    </div>
  {:else}
    <div class="grid" aria-label="Browsers">
      {#each browsers as browser, i (browser.id)}
        <BrowserTile
          {browser}
          index={i}
          highlighted={i === highlighted}
          onpick={() => pick(browser, null)}
          onmenu={(anchor) => {
            highlighted = i;
            menu = { browser, anchor };
          }}
        />
      {/each}
    </div>
  {/if}

  <footer class="row">
    {#if canPick && context?.settings.offer_remember}
      <label class="row remember">
        <input type="checkbox" bind:checked={remember} />
        <span>Always use for <strong>{context?.url?.host}</strong></span>
      </label>
    {/if}
    <span class="grow"></span>
    <span class="hint muted">1–9 · arrows · Enter · Esc</span>
    <button type="button" title="Settings (,)" aria-label="Settings" onclick={() => api.openSettings()}>⚙</button>
    <button type="button" onclick={() => api.dismiss()}>Cancel</button>
  </footer>
</main>

{#if menu}
  <LaunchMenu
    browser={menu.browser}
    anchor={menu.anchor}
    onchoose={(launch) => {
      const b = menu?.browser;
      menu = null;
      if (b) void pick(b, launch);
    }}
    onclose={() => (menu = null)}
  />
{/if}

<style>
  .picker {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px 16px 10px;
  }
  .strip {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--gradient);
  }
  header {
    min-height: 40px;
  }
  .host {
    font-weight: 800;
    font-size: 18px;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .href {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: text;
  }
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-auto-rows: minmax(100px, auto);
    gap: 12px;
    align-content: start;
    overflow-y: auto;
    padding: 2px;
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    text-align: center;
  }
  footer {
    font-size: 12px;
  }
  .remember {
    gap: 6px;
  }
  .hint {
    font-size: 11px;
  }
</style>
