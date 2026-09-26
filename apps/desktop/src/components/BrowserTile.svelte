<script lang="ts">
  import type { BrowserView } from "../lib/types";
  import { shortcutFor } from "../lib/keys";
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
  const shortcut = $derived(shortcutFor(index));

  $effect(() => {
    if (highlighted) element?.focus({ preventScroll: false });
  });
</script>

<div class="cell" class:highlighted>
  <button
    bind:this={element}
    class="tile"
    type="button"
    aria-label={`${browser.name}${browser.launches.length ? ", has launch options" : ""}`}
    onclick={onpick}
    oncontextmenu={(e) => {
      e.preventDefault();
      if (browser.launches.length && element) onmenu(element);
    }}
  >
    {#if shortcut}<span class="badge">{shortcut}</span>{/if}
    {#if browser.icon}
      <img class="icon" src={browser.icon} alt="" draggable="false" />
    {:else}
      <span class="icon letter" style:--tile-accent={accent}>{initial}</span>
    {/if}
    <span class="name" title={browser.name}>{shown.base}</span>
    {#if shown.mode}<span class="mode">{shown.mode}</span>{/if}
  </button>
  {#if browser.launches.length}
    <!-- Beside the tile rather than inside it: a button cannot contain
         another one. Left out of the tab order because Space or the
         context-menu key already open this menu from the highlighted tile. -->
    <button
      class="more"
      type="button"
      tabindex="-1"
      title="Launch options"
      aria-label={`Launch options for ${browser.name}`}
      onclick={() => {
        if (element) onmenu(element);
      }}>▾</button
    >
  {/if}
</div>

<style>
  /* The cell holds the tile and its chevron, so both lift together. */
  .cell {
    position: relative;
    display: grid;
    min-width: 0;
    transition: transform 80ms ease;
  }
  .cell:hover,
  .cell.highlighted {
    transform: translateY(-1px);
  }
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 14px 8px 10px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    min-width: 0;
    transition: border-color 80ms ease;
  }
  .cell:hover .tile,
  .cell.highlighted .tile {
    border-color: var(--accent);
  }
  .badge {
    position: absolute;
    top: 6px;
    left: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .icon {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    object-fit: contain;
  }
  .letter {
    display: grid;
    place-items: center;
    font-weight: 700;
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
  }
  .mode {
    margin-top: -6px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .more {
    position: absolute;
    top: 4px;
    right: 6px;
    border: none;
    background: transparent;
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
