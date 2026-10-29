<script lang="ts">
  import { onMount, tick } from "svelte";
  import { setLanguage, t, tParts, tRust } from "../i18n/i18n.svelte";
  import { api, asMessage, describeError } from "../lib/api";
  import { copyText } from "../lib/clipboard";
  import { columnCount, isMac, mapKey, moveIndex } from "../lib/keys";
  import { applyTheme } from "../lib/theme";
  import type { BrowserView, LaunchContext, UpdateStatus } from "../lib/types";
  import Banner from "../components/Banner.svelte";
  import BrowserTile from "../components/BrowserTile.svelte";
  import LaunchMenu from "../components/LaunchMenu.svelte";

  /** `⌘` is the command key, rather than `Ctrl`. */
  const mac = isMac(navigator.platform);

  let context = $state<LaunchContext | null>(null);
  let error = $state<string | null>(null);
  let highlighted = $state(0);
  let remember = $state(false);
  let busy = $state(false);
  let menu = $state<{ browser: BrowserView; anchor: HTMLElement } | null>(null);
  let gridWidth = $state(560);

  /** The link is being edited: a field stands in its line, holding `draft`. */
  let editing = $state(false);
  let draft = $state("");
  let field = $state<HTMLInputElement | undefined>();

  /** Where the running version stands: from Reroute's own check at start-up,
   * then from the update button. It colours that button. */
  let status = $state<UpdateStatus>({ state: "unknown" });
  /** The button was pressed: its result replaces the key hint. */
  let showStatus = $state(false);
  let checking = $state(false);

  const updateTitle = $derived.by(() => {
    switch (status.state) {
      case "current":
        return t("picker.update.current", { version: status.version });
      case "available":
        return t("picker.update.available", { version: status.version });
      case "failed":
        return t("picker.update.failedReason", { reason: tRust(status.reason) });
      default:
        return t("picker.update.check");
    }
  });

  /** The picker's keys, with the system's names for its modifiers. */
  const hint = $derived(t(mac ? "picker.hint.mac" : "picker.hint"));

  const browsers = $derived(context?.browsers ?? []);
  const layout = $derived(context?.settings.picker_layout ?? "tiles");
  const columns = $derived(columnCount(layout, gridWidth));
  const canPick = $derived(!!context?.url && browsers.length > 0);

  /** Read the pending link, then ask to be shown. A picker kept loaded in
   * the background still displays the previous link until this point, so
   * the window only appears once the new one is on screen. */
  async function load() {
    try {
      context = await api.launchContext();
      applyTheme(context.settings.theme);
      setLanguage(context.settings.language);
      highlighted = 0;
      remember = false;
      menu = null;
      busy = false;
      editing = false;
      error = null;
      showStatus = false;
      if (!checking) status = context.update;
      await tick();
    } catch (e) {
      error = describeError(e);
    }
    void api.showPicker();
  }

  onMount(() => {
    void load();
    const unlistenContext = api.onContextChanged(() => void load());
    const unlistenStatus = api.onUpdateStatus((next) => {
      if (!checking) status = next;
    });
    return () => {
      void unlistenContext.then((f) => f());
      void unlistenStatus.then((f) => f());
    };
  });

  /** Open the link in `browser`, or in its private entry when `privately`
   * and it has one (Shift); a launch option is for the browser itself. */
  async function pick(browser: BrowserView, launch: string | null, privately = false) {
    if (busy || !context?.url) return;
    const target = privately && browser.private ? browser.private : browser.id;
    busy = true;
    error = null;
    try {
      await api.pick(context.url.href, target, target === browser.id ? launch : null, remember);
    } catch (e) {
      error = describeError(e);
      busy = false;
    }
  }

  /** Copy the link on screen and close the picker, which stays open, saying
   * so, when the copy failed. */
  async function copyLink() {
    if (busy || !context?.url) return;
    try {
      await copyText(context.url.href);
    } catch {
      error = t("picker.copyFailed");
      return;
    }
    void api.dismiss();
  }

  async function startEditing() {
    if (busy || !context?.url) return;
    draft = context.url.href;
    editing = true;
    error = null;
    await tick();
    field?.focus();
    field?.select();
  }

  function stopEditing() {
    editing = false;
    error = null;
  }

  /** Rust checks the edited link as any incoming one; a refusal shows, and
   * the field stays for another try. */
  async function applyEdit() {
    if (!context?.url) return;
    if (draft.trim() === context.url.href) {
      stopEditing();
      return;
    }
    try {
      context.url = await api.editLink(context.url.href, draft);
      stopEditing();
    } catch (e) {
      error = describeError(e);
    }
  }

  function onfieldkeydown(e: KeyboardEvent) {
    // Every key is the field's: none reaches the picker's shortcuts.
    e.stopPropagation();
    if (e.isComposing) return;
    if (e.key === "Enter") {
      e.preventDefault();
      void applyEdit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      stopEditing();
    }
  }

  /** Text the user selected on the page, which `Ctrl+C` copies as usual. */
  function hasSelection(): boolean {
    return (window.getSelection()?.toString() ?? "") !== "";
  }

  /** Ask now which version is newest (the update button). */
  async function checkForUpdate() {
    if (checking) return;
    checking = true;
    showStatus = true;
    try {
      const release = await api.checkLatestRelease();
      status = release.newer
        ? { state: "available", version: release.version }
        : { state: "current", version: release.version };
    } catch (e) {
      status = { state: "failed", reason: asMessage(e) };
    } finally {
      checking = false;
    }
  }

  async function openDownloadPage() {
    try {
      await api.openReleasePage();
    } catch (e) {
      error = describeError(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (menu) return;
    // While the link is edited the picker's keys wait, even with the focus
    // gone from the field; Escape still gives the old link back.
    if (editing) {
      if (e.key === "Escape") stopEditing();
      return;
    }
    const action = mapKey(e, columns, mac);
    if (action.kind === "none" || (action.kind === "copy" && hasSelection())) return;
    e.preventDefault();
    switch (action.kind) {
      case "select": {
        const b = browsers[action.index];
        if (b) void pick(b, null, action.private);
        break;
      }
      case "move":
        highlighted = moveIndex(highlighted, action.delta, browsers.length);
        break;
      case "confirm": {
        const b = browsers[highlighted];
        if (b) void pick(b, null, action.private);
        break;
      }
      case "copy":
        void copyLink();
        break;
      case "edit":
        void startEditing();
        break;
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
  <header>
    {#if context?.url}
      <div class="row">
        <div class="address">
          <div class="host" title={context.url.href}>{context.url.host}</div>
          {#if editing}
            <input
              bind:this={field}
              bind:value={draft}
              class="edit"
              type="url"
              spellcheck="false"
              autocomplete="off"
              aria-label={t("picker.edit.field")}
              onkeydown={onfieldkeydown}
            />
          {:else}
            <div class="href muted" title={context.url.href}>{context.url.href}</div>
          {/if}
        </div>
        <!-- Drawn rather than text symbols, as the update button below. -->
        <button
          type="button"
          class="tool"
          title={t(mac ? "picker.copy.title.mac" : "picker.copy.title")}
          aria-label={t("picker.copy")}
          disabled={busy}
          onclick={copyLink}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <rect x="9" y="9" width="11" height="11" rx="2" />
            <path d="M5 15V6a2 2 0 0 1 2-2h8" />
          </svg>
        </button>
        <button
          type="button"
          class="tool"
          class:active={editing}
          title={t(mac ? "picker.edit.title.mac" : "picker.edit.title")}
          aria-label={t("picker.edit")}
          aria-pressed={editing}
          disabled={busy}
          onclick={() => (editing ? stopEditing() : void startEditing())}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 20h4L19 9l-4-4L4 16z" />
            <path d="M13 7l4 4" />
          </svg>
        </button>
      </div>
    {:else if context?.url_error}
      <Banner tone="error">{t("picker.refused", { reason: tRust(context.url_error) })}</Banner>
    {:else}
      <div class="host" translate="no">Reroute</div>
      <div class="href muted">{t("picker.noLink")}</div>
    {/if}
  </header>

  {#if context?.config_error}
    <Banner tone="error">{t("picker.configError", { reason: tRust(context.config_error) })}</Banner>
  {/if}
  {#if error}
    <Banner tone="error">{error}</Banner>
  {/if}

  {#if context && browsers.length === 0}
    <div class="empty">
      <p>{t("picker.empty")}</p>
      <button class="primary" type="button" onclick={() => api.openSettings()}>{t("picker.openSettings")}</button>
    </div>
  {:else}
    <div
      class="grid"
      class:list={layout === "list"}
      class:two-columns={layout === "two-columns"}
      bind:clientWidth={gridWidth}
      aria-label={t("picker.browsers")}
    >
      {#each browsers as browser, i (browser.id)}
        <BrowserTile
          {browser}
          index={i}
          row={layout !== "tiles"}
          highlighted={i === highlighted}
          onpick={(e) => pick(browser, null, e.shiftKey)}
          onmenu={(anchor) => {
            highlighted = i;
            menu = { browser, anchor };
          }}
        />
      {/each}
    </div>
  {/if}

  <footer class="row">
    <!-- A remembered choice is a rule, so there is nothing to offer while rules are off. -->
    {#if canPick && context?.settings.offer_remember && context.settings.rules_enabled}
      <label class="row remember">
        <input type="checkbox" bind:checked={remember} />
        <!-- On one line: whitespace between the parts would show. -->
        <span>{#each tParts("picker.remember", { host: context.url?.host ?? "" }) as part}{#if part.slot === "host"}<strong>{part.text}</strong>{:else}{part.text}{/if}{/each}</span>
      </label>
    {/if}
    <span class="grow"></span>
    <!-- The update button reports where the key hint was. -->
    <span class="hint muted" role="status">
      {#if !showStatus}
        <span title={hint}>{hint}</span>
      {:else if checking}
        {t("picker.update.checking")}
      {:else if status.state === "failed"}
        <span title={tRust(status.reason)}>{t("picker.update.failed")}</span>
      {:else if status.state === "available"}
        <button type="button" class="link" onclick={openDownloadPage}>{t("picker.update.available", { version: status.version })}</button>
      {:else if status.state === "current"}
        {t("picker.update.current", { version: status.version })}
      {/if}
    </span>
    <!-- Coloured by what the last check found: green when up to date, the
         accent colour when a newer version is out, red when it failed. -->
    <button
      type="button"
      class="icon"
      class:current={status.state === "current"}
      class:available={status.state === "available"}
      class:failed={status.state === "failed"}
      title={updateTitle}
      aria-label={t("picker.update.check")}
      disabled={checking}
      onclick={checkForUpdate}
    >
      <!-- Drawn rather than a text symbol: WebKit would search the fonts for one. -->
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M20 12a8 8 0 1 1-2.34-5.66L20 8" />
        <path d="M20 3v5h-5" />
      </svg>
    </button>
    <button type="button" title={t("picker.settings.title")} aria-label={t("picker.settings")} onclick={() => api.openSettings()}>⚙</button>
    <button type="button" onclick={() => api.dismiss()}>{t("picker.cancel")}</button>
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
    padding: 12px 14px;
  }
  header {
    min-height: 40px;
  }
  .address {
    flex: 1;
    min-width: 0;
  }
  /* The field takes the place of the address line, at the same height. */
  .edit {
    padding: 0 6px;
    font-size: 12px;
    line-height: 1.45;
    border-radius: 5px;
  }
  /* The link's buttons, beside it. */
  .tool {
    display: inline-grid;
    place-items: center;
    align-self: flex-start;
    padding: 5px 7px;
  }
  .tool.active {
    color: var(--accent);
    border-color: var(--accent);
  }
  .host {
    font-weight: 600;
    font-size: 15px;
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
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 10px;
    align-content: start;
    overflow-y: auto;
    padding: 2px;
  }
  /* The lists: one browser per line, in one column or two. */
  .grid.list {
    grid-template-columns: minmax(0, 1fr);
    gap: 6px;
  }
  .grid.two-columns {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
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
  /* Cut short rather than pushing the buttons out when room runs out; its
     tooltip holds it whole. */
  .hint {
    min-width: 0;
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .icon {
    display: inline-grid;
    place-items: center;
  }
  .icon.current {
    color: var(--on-fill);
    background: var(--success-fill);
    border-color: var(--success);
  }
  .icon.available {
    color: var(--accent);
    border-color: var(--accent);
  }
  .icon.failed {
    color: var(--danger);
  }
  .icon svg,
  .tool svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    font-size: inherit;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
