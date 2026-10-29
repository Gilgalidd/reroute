<script lang="ts">
  import { t, tParts } from "../i18n/i18n.svelte";
  import { api, describeError } from "../lib/api";
  import { normalizeConfig } from "../lib/config";
  import { newId } from "../lib/ids";
  import { splitLines } from "../lib/lines";
  import type { Browser, Config } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable() }: { config: Config } = $props();

  let selectedId = $state<string | null>(null);
  let message = $state<{ tone: "info" | "error"; text: string } | null>(null);

  const selected = $derived(config.browsers.find((b) => b.id === selectedId) ?? null);
  const selectedIndex = $derived(config.browsers.findIndex((b) => b.id === selectedId));

  function addBrowser() {
    const browser: Browser = { id: newId(), name: t("browsers.newName"), path: "", args: ["%URL%"], hidden: false, launches: [] };
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

  /** Add the installed browsers to the draft; Save keeps them, like any edit. */
  async function detect() {
    message = null;
    try {
      const before = config.browsers.length;
      config = normalizeConfig(await api.discoverBrowsers($state.snapshot(config)));
      const added = config.browsers.length - before;
      message = {
        tone: "info",
        text: added ? t("browsers.detected", { count: added }) : t("browsers.noneFound"),
      };
    } catch (e) {
      message = { tone: "error", text: describeError(e) };
    }
  }
</script>

<div class="two-col">
  <aside>
    <div class="row">
      <button type="button" class="primary" onclick={addBrowser}>{t("browsers.add")}</button>
      <button type="button" onclick={detect}>{t("browsers.detect")}</button>
    </div>
    <ul class="list" role="listbox" aria-label={t("browsers.list")}>
      {#each config.browsers as browser (browser.id)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={browser.id === selectedId}
            class:active={browser.id === selectedId}
            onclick={() => (selectedId = browser.id)}
          >
            <span class="grow">{browser.name || t("browsers.unnamed")}</span>
            {#if browser.hidden}<span class="muted">{t("browsers.hidden")}</span>{/if}
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
        <span>{t("browsers.name")}</span>
        <input type="text" bind:value={selected.name} />
      </label>
      <label>
        <span>{t("browsers.path")}</span>
        <input type="text" bind:value={selected.path} placeholder="/usr/bin/firefox" spellcheck="false" translate="no" />
      </label>
      <label>
        <!-- On one line: whitespace between the parts would show. -->
        <span>{#each tParts("browsers.args", { url: "%URL%" }) as part}{#if part.slot === "url"}<code>{part.text}</code>{:else}{part.text}{/if}{/each}</span>
        <textarea rows="3" value={selected.args.join("\n")} oninput={(e) => (selected.args = splitLines(e.currentTarget.value))}></textarea>
      </label>
      <label>
        <span>{t("browsers.icon")}</span>
        <input type="text" value={selected.icon ?? ""} oninput={(e) => (selected.icon = e.currentTarget.value.trim() || null)} spellcheck="false" />
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={selected.hidden} />
        <span>{t("browsers.hide")}</span>
      </label>

      <h3>{t("browsers.launches")}</h3>
      <p class="muted">{t("browsers.launches.help")}</p>
      {#each selected.launches as launch, i (launch.id)}
        <div class="launch">
          <input type="text" bind:value={launch.name} placeholder={t("browsers.launch.namePlaceholder")} aria-label={t("browsers.launch.name")} />
          <textarea
            rows="2"
            value={launch.args.join("\n")}
            placeholder={"--profile-directory=Profile 1\n%URL%"}
            aria-label={t("browsers.launch.args")}
            spellcheck="false"
            translate="no"
            oninput={(e) => (launch.args = splitLines(e.currentTarget.value))}
          ></textarea>
          <button type="button" class="danger" aria-label={t("browsers.launch.remove")} onclick={() => selected.launches.splice(i, 1)}>✕</button>
        </div>
      {/each}
      <button type="button" onclick={() => selected.launches.push({ id: newId(), name: t("browsers.launch.newName"), args: ["%URL%"] })}
        >{t("browsers.launch.add")}</button
      >

      <div class="row actions">
        <button type="button" onclick={() => move(-1)} disabled={selectedIndex <= 0}>{t("browsers.moveUp")}</button>
        <button type="button" onclick={() => move(1)} disabled={selectedIndex >= config.browsers.length - 1}>{t("browsers.moveDown")}</button>
        <span class="grow"></span>
        <button type="button" class="danger" onclick={remove}>{t("browsers.remove")}</button>
      </div>
    {:else}
      <p class="muted">{t("browsers.none")}</p>
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
    display: grid;
    grid-template-columns: 1fr 1.5fr auto;
    gap: 6px;
    align-items: start;
  }
  .actions {
    margin-top: 8px;
  }
</style>
