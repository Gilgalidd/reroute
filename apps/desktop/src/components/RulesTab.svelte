<script lang="ts">
  import { api, describeError } from "../lib/api";
  import { splitLines } from "../lib/lines";
  import { patternProblem } from "../lib/patterns";
  import type { Config, Ruleset, TestResult } from "../lib/types";
  import Banner from "./Banner.svelte";

  let { config = $bindable() }: { config: Config } = $props();

  let testInput = $state("");
  let testResult = $state<TestResult | null>(null);
  let testError = $state<string | null>(null);

  function add() {
    const first = config.browsers[0];
    if (!first) return;
    const set: Ruleset = { name: "New ruleset", browser: first.id, launch: null, patterns: [] };
    config.rulesets.unshift(set);
  }

  function move(i: number, delta: number) {
    const to = i + delta;
    if (to < 0 || to >= config.rulesets.length) return;
    const [item] = config.rulesets.splice(i, 1);
    if (item) config.rulesets.splice(to, 0, item);
  }

  /** Edit ruleset `i` in place. Element-level `bind:` inside `{#each}` is
   * avoided on purpose: writing through the bound `config` prop is explicit
   * and works the same whatever the compiler decides about the prop. */
  function update(i: number, patch: Partial<Ruleset>) {
    const set = config.rulesets[i];
    if (set) Object.assign(set, patch);
  }

  function launchesOf(browserId: string) {
    return config.browsers.find((b) => b.id === browserId)?.launches ?? [];
  }

  function problems(patterns: string[]): string[] {
    return patterns.map((p) => patternProblem(p)).filter((p): p is string => p !== null);
  }

  async function runTest() {
    testError = null;
    try {
      testResult = await api.testUrl(testInput);
    } catch (e) {
      testError = describeError(e);
    }
  }
</script>

<div class="rules">
  <div class="row">
    <button type="button" class="primary" onclick={add} disabled={config.browsers.length === 0}>Add ruleset</button>
    <span class="muted">Evaluated top to bottom; the first matching pattern wins.</span>
  </div>

  {#if !config.settings.rules_enabled}
    <Banner tone="info">Rules are currently disabled in General settings; the picker always asks.</Banner>
  {/if}

  {#each config.rulesets as set, i}
    <fieldset class="card">
      <div class="row head">
        <input
          type="text"
          class="grow"
          value={set.name}
          placeholder="Ruleset name"
          oninput={(e) => update(i, { name: e.currentTarget.value })}
        />
        <select
          value={set.browser}
          aria-label="Browser"
          onchange={(e) => update(i, { browser: e.currentTarget.value, launch: null })}
        >
          {#each config.browsers as b (b.id)}
            <option value={b.id}>{b.name}</option>
          {/each}
        </select>
        <select
          value={set.launch ?? ""}
          aria-label="Launch option"
          onchange={(e) => update(i, { launch: e.currentTarget.value || null })}
        >
          <option value="">Default launch</option>
          {#each launchesOf(set.browser) as l (l.id)}
            <option value={l.id}>{l.name}</option>
          {/each}
        </select>
        <button type="button" aria-label="Move up" onclick={() => move(i, -1)} disabled={i === 0}>↑</button>
        <button type="button" aria-label="Move down" onclick={() => move(i, 1)} disabled={i === config.rulesets.length - 1}>↓</button>
        <button type="button" class="danger" aria-label="Delete ruleset" onclick={() => config.rulesets.splice(i, 1)}>✕</button>
      </div>
      <textarea
        rows="3"
        spellcheck="false"
        aria-label="Patterns, one per line"
        placeholder={"domain:*.github.com\nregex:^https://open\\.spotify\\.com/\nexact:https://example.com/login"}
        value={set.patterns.join("\n")}
        oninput={(e) => update(i, { patterns: splitLines(e.currentTarget.value) })}
      ></textarea>
      {#each problems(set.patterns) as problem (problem)}
        <span class="problem">{problem}</span>
      {/each}
    </fieldset>
  {/each}

  <details class="card" open>
    <summary>Try a URL</summary>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        void runTest();
      }}
    >
      <input type="text" class="grow" bind:value={testInput} placeholder="https://…" spellcheck="false" />
      <button type="submit">Test saved rules</button>
    </form>
    <p class="muted">Tests the rules as last saved.</p>
    {#if testError}
      <Banner tone="error">{testError}</Banner>
    {:else if testResult?.error}
      <Banner tone="error">Refused: {testResult.error}</Banner>
    {:else if testResult?.matched}
      <Banner tone="success">
        Opens in <strong>{testResult.matched.browser}</strong>{testResult.matched.launch ? ` (${testResult.matched.launch})` : ""}
        via <code>{testResult.matched.pattern}</code> in “{testResult.matched.ruleset}”.
      </Banner>
    {:else if testResult}
      <Banner tone="info">No rule matches; the picker would ask.</Banner>
    {/if}
  </details>
</div>

<style>
  .rules {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
  }
  .head select {
    width: auto;
    max-width: 180px;
  }
  .problem {
    color: var(--danger);
    font-size: 12px;
  }
  summary {
    cursor: pointer;
    font-weight: 600;
  }
</style>
