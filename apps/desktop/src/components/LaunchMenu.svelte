<script lang="ts">
  import type { BrowserView } from "../lib/types";

  let {
    browser,
    anchor,
    onchoose,
    onclose,
  }: {
    browser: BrowserView;
    anchor: HTMLElement;
    onchoose: (launch: string | null) => void;
    onclose: () => void;
  } = $props();

  let menu = $state<HTMLDivElement | undefined>();
  let active = $state(0);
  const items = $derived([{ id: null as string | null, name: `${browser.name} (default)` }, ...browser.launches]);

  const position = $derived.by(() => {
    const rect = anchor.getBoundingClientRect();
    return {
      left: Math.min(rect.left, window.innerWidth - 240),
      top: Math.min(rect.bottom + 4, window.innerHeight - 40),
    };
  });

  $effect(() => {
    menu?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") onclose();
    else if (e.key === "ArrowDown") active = (active + 1) % items.length;
    else if (e.key === "ArrowUp") active = (active - 1 + items.length) % items.length;
    else if (e.key === "Enter") onchoose(items[active]?.id ?? null);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose} onkeydown={onkeydown}></div>
<div
  class="menu"
  role="menu"
  tabindex="-1"
  bind:this={menu}
  style:left={`${position.left}px`}
  style:top={`${position.top}px`}
  onkeydown={onkeydown}
>
  {#each items as item, i (item.id ?? "default")}
    <button
      type="button"
      role="menuitem"
      class:active={i === active}
      onmouseenter={() => (active = i)}
      onclick={() => onchoose(item.id)}
    >
      {item.name}
    </button>
  {/each}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
  }
  .menu {
    position: fixed;
    min-width: 200px;
    max-width: 240px;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    z-index: 10;
  }
  .menu button {
    text-align: left;
    border: none;
    background: transparent;
    border-radius: 6px;
    padding: 6px 10px;
  }
  .menu button.active {
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
