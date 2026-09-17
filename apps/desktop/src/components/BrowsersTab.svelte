<script lang="ts">
  import { api, describeError } from "../lib/api";
  import { newId } from "../lib/ids";
  import type { Browser, Config } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable(), onreplace }: { config: Config; onreplace: (c: Config) => void } = $props();

  let selectedId = $state<string | null>(null);
  let message = $state<{ tone: "info" | "error"; text: string } | null>(null);

  const selected = $derived(config.browsers.find((b) => b.id === selectedId) ?? null);
  const selectedIndex = $derived(config.browsers.findIndex((b) => b.id === selectedId));

  function addBrowser() {
    const browser: Browser = { id: newId(), name: "New browser", path: "", args: ["%URL%"], hidden: false, launches: [] };
    config.browsers.push(browser);
    selectedId = browser.id;
  }

  function remove() {
    if (selectedIndex < 0) return;
    const id = selectedId;
    config.browsers.splice(selectedIndex, 1);
    config.rulesets = config.rulesets.filter((r) => r.browser !== id);
    selectedId = config.browsers[0]?.id ?? null;
  }

  function move(delta: number) {
    const from = selectedIndex;
    const to = from + delta;
    if (from < 0 || to < 0 || to >= config.browsers.length) return;
    const [item] = config.browsers.splice(from, 1);
    if (item) config.browsers.splice(to, 0, item);
  }

  async function detect() {
    message = null;
    try {
      const before = config.browsers.length;
      const fresh = await api.discoverBrowsers();
      onreplace(fresh);
      message = { tone: "info", text: `${fresh.browsers.length - before} new browser(s) added.` };
    } catch (e) {
      message = { tone: "error", text: describeError(e) };
    }
  }

  function linesToArgs(text: string): string[] {
    return text.split(/\r?\n/).map((l) => l.trim()).filter((l) => l.length > 0);
  }
</script>

<div class="two-col">
  <aside>
    <div class="row">
      <button type="button" class="primary" onclick={addBrowser}>Add</button>
      <button type="button" onclick={detect}>Detect installed</button>
    </div>
    <ul class="list" role="listbox" aria-label="Browsers">
      {#each config.browsers as browser (browser.id)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={browser.id === selectedId}
            class:active={browser.id === selectedId}
            onclick={() => (selectedId = browser.id)}
          >
            <span class="grow">{browser.name || "(unnamed)"}</span>
            {#if browser.hidden}<span class="muted">hidden</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if message}
      <Banner tone={message.tone}>{message.text}</Banner>
    {/if}
  </aside>

  <div class="editor">
    {#if selected}
      <label>
        <span>Name</span>
        <input type="text" bind:value={selected.name} />
      </label>
      <label>
        <span>Executable (absolute path)</span>
        <input type="text" bind:value={selected.path} placeholder="/usr/bin/firefox" spellcheck="false" />
      </label>
      <label>
        <span>Arguments, one per line. <code>%URL%</code> marks where the link goes (appended if absent).</span>
        <textarea rows="3" value={selected.args.join("\n")} oninput={(e) => (selected.args = linesToArgs(e.currentTarget.value))}></textarea>
      </label>
      <label>
        <span>Icon file (PNG, SVG or ICO; optional)</span>
        <input type="text" value={selected.icon ?? ""} oninput={(e) => (selected.icon = e.currentTarget.value.trim() || null)} spellcheck="false" />
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={selected.hidden} />
        <span>Hide from the picker (rules can still use it)</span>
      </label>

      <h3>Launch options</h3>
      <p class="muted">Profiles, private windows… shown on right-click in the picker.</p>
      {#each selected.launches as launch, i (launch.id)}
        <div class="launch row">
          <input type="text" class="grow" bind:value={launch.name} placeholder="Private window" />
          <input
            type="text"
            class="grow"
            value={launch.args.join(" ")}
            placeholder="--private-window %URL%"
            spellcheck="false"
            oninput={(e) => (launch.args = e.currentTarget.value.split(/\s+/).filter((a) => a.length > 0))}
          />
          <button type="button" class="danger" aria-label="Remove launch option" onclick={() => selected.launches.splice(i, 1)}>✕</button>
        </div>
      {/each}
      <button type="button" onclick={() => selected.launches.push({ id: newId(), name: "New option", args: ["%URL%"] })}>Add launch option</button>

      <div class="row actions">
        <button type="button" onclick={() => move(-1)} disabled={selectedIndex <= 0}>Move up</button>
        <button type="button" onclick={() => move(1)} disabled={selectedIndex >= config.browsers.length - 1}>Move down</button>
        <span class="grow"></span>
        <button type="button" class="danger" onclick={remove}>Remove browser</button>
      </div>
    {:else}
      <p class="muted">Select a browser, add one, or detect the browsers installed on this computer.</p>
    {/if}
  </div>
</div>

<style>
  .two-col {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 20px;
    height: 100%;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
  }
  .list button {
    width: 100%;
    display: flex;
    gap: 6px;
    text-align: left;
    border: none;
    background: transparent;
  }
  .list button.active {
    background: var(--surface-2);
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label.row {
    flex-direction: row;
  }
  h3 {
    margin: 8px 0 0;
    font-size: 14px;
  }
  .launch {
    gap: 6px;
  }
  .actions {
    margin-top: 8px;
  }
</style>
