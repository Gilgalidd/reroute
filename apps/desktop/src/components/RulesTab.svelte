<script lang="ts">
  import { t, tParts, tRust } from "../i18n/i18n.svelte";
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
    const set: Ruleset = { name: t("rules.newName"), browser: first.id, launch: null, patterns: [] };
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

  /** One line per faulty pattern, naming it: two patterns can share a fault. */
  function problems(patterns: string[]): string[] {
    return patterns.flatMap((pattern) => {
      const problem = patternProblem(pattern);
      return problem ? [t("rules.problem", { pattern, problem: t(problem) })] : [];
    });
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
    <button type="button" class="primary" onclick={add} disabled={config.browsers.length === 0}>{t("rules.add")}</button>
    <span class="muted">{t("rules.order")}</span>
  </div>

  {#if !config.settings.rules_enabled}
    <Banner tone="info">{t("rules.disabled")}</Banner>
  {/if}

  {#each config.rulesets as set, i}
    <fieldset class="card">
      <div class="row head">
        <input
          type="text"
          class="grow"
          value={set.name}
          placeholder={t("rules.name")}
          oninput={(e) => update(i, { name: e.currentTarget.value })}
        />
        <select
          value={set.browser}
          aria-label={t("rules.browser")}
          onchange={(e) => update(i, { browser: e.currentTarget.value, launch: null })}
        >
          {#each config.browsers as b (b.id)}
            <option value={b.id}>{b.name}</option>
          {/each}
        </select>
        <select
          value={set.launch ?? ""}
          aria-label={t("rules.launch")}
          onchange={(e) => update(i, { launch: e.currentTarget.value || null })}
        >
          <option value="">{t("rules.launch.default")}</option>
          {#each launchesOf(set.browser) as l (l.id)}
            <option value={l.id}>{l.name}</option>
          {/each}
        </select>
        <button type="button" aria-label={t("rules.moveUp")} onclick={() => move(i, -1)} disabled={i === 0}>↑</button>
        <button type="button" aria-label={t("rules.moveDown")} onclick={() => move(i, 1)} disabled={i === config.rulesets.length - 1}>↓</button>
        <button type="button" class="danger" aria-label={t("rules.delete")} onclick={() => config.rulesets.splice(i, 1)}>✕</button>
      </div>
      <textarea
        rows="3"
        spellcheck="false"
        translate="no"
        aria-label={t("rules.patterns")}
        placeholder={"domain:*.github.com\nregex:^https://open\\.spotify\\.com/\nexact:https://example.com/login"}
        value={set.patterns.join("\n")}
        oninput={(e) => update(i, { patterns: splitLines(e.currentTarget.value) })}
      ></textarea>
      <!-- Not keyed by text: the same pattern may be typed twice. -->
      {#each problems(set.patterns) as problem}
        <span class="problem">{problem}</span>
      {/each}
    </fieldset>
  {/each}

  <details class="card" open>
    <summary>{t("rules.test")}</summary>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        void runTest();
      }}
    >
      <input type="text" class="grow" bind:value={testInput} placeholder="https://…" spellcheck="false" translate="no" />
      <button type="submit">{t("rules.test.run")}</button>
    </form>
    <p class="muted">{t("rules.test.saved")}</p>
    {#if testError}
      <Banner tone="error">{testError}</Banner>
    {:else if testResult?.error}
      <Banner tone="error">{t("rules.test.refused", { reason: tRust(testResult.error) })}</Banner>
    {:else if testResult?.matched}
      {@const { browser, launch, pattern, ruleset } = testResult.matched}
      <!-- On one line: whitespace between the parts would show. -->
      <Banner tone="success">{#each tParts(launch ? "rules.test.matchLaunch" : "rules.test.match", { browser, launch: launch ?? "", pattern, ruleset }) as part}{#if part.slot === "browser"}<strong>{part.text}</strong>{:else if part.slot === "pattern"}<code>{part.text}</code>{:else}{part.text}{/if}{/each}</Banner>
    {:else if testResult}
      <Banner tone="info">{t("rules.test.none")}</Banner>
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
