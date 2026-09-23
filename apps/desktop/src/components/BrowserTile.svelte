<script lang="ts">
  import type { BrowserView } from "../lib/types";
  import { splitName } from "../lib/names";

  let {
    browser,
    index,
    highlighted,
    onpick,
    onmenu,
  }: {
    browser: BrowserView;
    index: number;
    highlighted: boolean;
    onpick: () => void;
    onmenu: (anchor: HTMLElement) => void;
  } = $props();

  let element = $state<HTMLButtonElement | undefined>();
  const shown = $derived(splitName(browser.name));
  const initial = $derived(shown.base.charAt(0).toUpperCase() || "?");
  const accent = $derived(browser.brand?.color ?? "#64748b");

  $effect(() => {
    if (highlighted) element?.focus({ preventScroll: false });
  });
</script>

<button
  bind:this={element}
  class="tile"
  class:highlighted
  type="button"
  aria-label={`${browser.name}${browser.launches.length ? ", has launch options" : ""}`}
  onclick={onpick}
  oncontextmenu={(e) => {
    e.preventDefault();
    if (browser.launches.length && element) onmenu(element);
  }}
>
  <span class="badge">{index + 1}</span>
  {#if browser.icon}
    <img class="icon" src={browser.icon} alt="" draggable="false" />
  {:else}
    <span class="icon letter" style:--tile-accent={accent}>{initial}</span>
  {/if}
  <span class="name" title={browser.name}>{shown.base}</span>
  {#if shown.mode}<span class="mode">{shown.mode}</span>{/if}
  {#if browser.launches.length}
    <span
      class="more"
      role="button"
      tabindex="-1"
      title="Launch options"
      onclick={(e) => {
        e.stopPropagation();
        if (element) onmenu(element);
      }}
      onkeydown={(e) => e.stopPropagation()}>▾</span
    >
  {/if}
</button>

<style>
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 16px 10px 10px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: 0 0 0 rgba(0, 0, 0, 0);
    min-width: 0;
    font-weight: 400;
    transition: transform var(--dur) var(--ease), box-shadow var(--dur) var(--ease),
      border-color var(--dur) var(--ease);
  }
  .tile:hover,
  .tile.highlighted {
    border-color: var(--accent);
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }
  .badge {
    position: absolute;
    top: 8px;
    left: 8px;
    width: 19px;
    height: 19px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--gradient);
    color: #ffffff;
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
  }
  .icon {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    object-fit: contain;
  }
  .letter {
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 20px;
    color: #fff;
    background: var(--tile-accent);
  }
  .name {
    width: 100%;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
    font-weight: 600;
  }
  .mode {
    margin-top: -4px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .more {
    position: absolute;
    top: 4px;
    right: 6px;
    color: var(--text-muted);
    font-size: 12px;
    padding: 2px 4px;
    border-radius: 6px;
  }
  .more:hover {
    background: var(--surface-2);
    color: var(--text);
  }
</style>
