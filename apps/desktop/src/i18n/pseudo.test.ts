// @vitest-environment jsdom
// Both windows, drawn in the pseudo-language: every text a user can see
// must come from the catalogue, so anything left in plain English was
// written in a component instead of `en.json`. See docs/translations.md.
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import Picker from "../routes/Picker.svelte";
import Settings from "../routes/Settings.svelte";
import { launchContext as englishContext, mockIpc, rustMessage, sparseConfig as englishConfig } from "../test/ipc";
import type { Config, LaunchContext, RustMessage } from "../lib/types";
import { setLanguage, t } from "./i18n.svelte";
import { PSEUDO, PSEUDO_CLOSE, PSEUDO_OPEN } from "./messages";

// The fixtures, with the pseudo-language as the `language` setting: both
// windows apply the setting they receive.
function launchContext(): LaunchContext {
  const context = englishContext();
  return { ...context, settings: { ...context.settings, language: PSEUDO } };
}

function sparseConfig(): Config {
  const config = englishConfig();
  return { ...config, settings: { ...config.settings, language: PSEUDO } };
}

/** A third party's words inside a message from Rust: they stay in English. */
const DETAIL = "TOML parse error at line 3";

/** What the fixtures put on screen as data: names, addresses, paths, details. */
const DATA = [
  "https://github.com/U-C-S/Hurl",
  "github.com",
  "Firefox",
  "Chromium",
  "Incognito",
  "Work",
  "/tmp/config.toml",
  "linux",
  "javascript",
  DETAIL,
];

/** A message from Rust with a reason inside it. */
const PATTERN_ERROR = rustMessage("config.pattern", {
  ruleset: "Work",
  problem: rustMessage("pattern.invalid_regex", { detail: DETAIL }),
});

const ATTRIBUTES = ["title", "aria-label", "placeholder"];

const open = PSEUDO_OPEN.replace(/[[\]]/g, "\\$&");
const close = PSEUDO_CLOSE.replace(/[[\]]/g, "\\$&");
/** The innermost pseudo-language string: a translated text can hold another one as a value. */
const MARKED = new RegExp(`${open}(?:(?!${open})[\\s\\S])*?${close}`, "g");

/** `text` without its pseudo-language strings and its data, if any letter is left. */
function leftover(text: string): string | null {
  let rest = text;
  for (let before = ""; before !== rest; ) {
    before = rest;
    rest = rest.replace(MARKED, " ");
  }
  for (const value of DATA) rest = rest.split(value).join(" ");
  return /[A-Za-z]/.test(rest) ? rest.replace(/\s+/g, " ").trim() : null;
}

/**
 * Texts and attributes on screen that are not in the pseudo-language.
 * `translate="no"` marks what is not language at all: the product's name,
 * example paths and patterns.
 */
function untranslated(): string[] {
  const found: string[] = [];
  const skip = (element: Element | null) => element?.closest('[translate="no"]') != null;
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let text = "";
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (!skip(node.parentElement)) text += `${node.textContent ?? ""}\n`;
  }
  const rest = leftover(text);
  if (rest) found.push(`text: ${rest}`);
  for (const element of document.body.querySelectorAll("*")) {
    for (const name of ATTRIBUTES) {
      const value = element.getAttribute(name);
      if (value === null || (name === "placeholder" && skip(element))) continue;
      const left = leftover(value);
      if (left) found.push(`${name}: ${left}`);
    }
  }
  return found;
}

describe("in the pseudo-language", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
    setLanguage(PSEUDO);
  });

  afterEach(() => {
    setLanguage("en");
  });

  describe("the picker", () => {
    it("translates its tiles, footer and launch menu", async () => {
      mockIpc({ launch_context: () => launchContext() });
      const { container } = render(Picker);
      await screen.findByText("github.com", { selector: ".host" });
      expect(untranslated()).toEqual([]);
      await fireEvent.click(container.querySelector("button.more")!);
      expect(screen.getByRole("menu")).toBeInTheDocument();
      expect(untranslated()).toEqual([]);
    });

    it("translates every answer of the update button", async () => {
      const answers = [
        () => ({ version: "9.9.9", newer: true, page: "https://example.org/" }),
        () => ({ version: "0.1.0", newer: false, page: "https://example.org/" }),
        () => {
          throw rustMessage("release.unreachable", { detail: DETAIL });
        },
      ];
      for (const answer of answers) {
        document.body.innerHTML = "";
        mockIpc({ launch_context: () => launchContext(), check_latest_release: answer });
        const { container } = render(Picker);
        await screen.findByText("github.com", { selector: ".host" });
        await fireEvent.click(container.querySelector("button.icon")!);
        await waitFor(() => expect(container.querySelector("button.icon")).not.toBeDisabled());
        expect(untranslated()).toEqual([]);
      }
    });

    it("translates a refused link, a configuration error and an empty list", async () => {
      mockIpc({
        launch_context: () => ({
          ...launchContext(),
          url: null,
          url_error: rustMessage("url.scheme_not_allowed", { scheme: "javascript" }),
          config_error: PATTERN_ERROR,
          browsers: [],
        }),
      });
      render(Picker);
      await screen.findAllByRole("alert");
      expect(untranslated()).toEqual([]);
    });
  });

  describe("the settings window", () => {
    function setup(loadError: RustMessage | null = null) {
      const ipc = mockIpc({
        get_config: () => ({ config: sparseConfig(), load_error: loadError }),
        app_info: () => ({ version: "0.1.0", config_path: "/tmp/config.toml", platform: "linux" }),
        default_browser_status: () => true,
        discover_browsers: (args) => args.config,
        register_default_browser: () => ({ kind: "needs_user_action", message: rustMessage("register.confirm_windows") }),
        test_url: () => ({
          normalized: "https://github.com/",
          error: null,
          matched: { ruleset: "Work", pattern: "domain:*.github.com", browser: "Chromium", launch: "Incognito" },
        }),
        import_hurl: (args) => ({
          config: args.config,
          browsers_added: 1,
          rulesets_added: 2,
          notes: [rustMessage("hurl.uwp", { browser: "Firefox" }), rustMessage("merge.browser_exists", { browser: "Chromium", existing: "Work" })],
        }),
        check_latest_release: () => ({ version: "9.9.9", newer: true, page: "https://example.org/" }),
      });
      render(Settings);
      return ipc;
    }

    async function tab(key: Parameters<typeof t>[0]) {
      await fireEvent.click(screen.getByRole("button", { name: t(key) }));
    }

    it("translates the browsers tab, and why config.toml was not read", async () => {
      setup(PATTERN_ERROR);
      await fireEvent.click(await screen.findByRole("option", { name: "Chromium" }));
      await fireEvent.click(screen.getByRole("button", { name: t("browsers.launch.add") }));
      await fireEvent.click(screen.getByRole("button", { name: t("browsers.detect") }));
      await screen.findByText(t("browsers.noneFound"));
      expect(untranslated()).toEqual([]);
    });

    it("translates the rules tab, its problems and its tester", async () => {
      setup();
      await screen.findByRole("option", { name: "Firefox" });
      await tab("settings.tab.general");
      await fireEvent.click(screen.getByLabelText(t("general.rulesEnabled")));
      await tab("settings.tab.rules");
      await fireEvent.input(screen.getByLabelText(t("rules.patterns")), {
        target: { value: "domain:\ndomain:bad host\nregex:(" },
      });
      await fireEvent.click(screen.getByRole("button", { name: t("rules.test.run") }));
      await screen.findByText("domain:*.github.com");
      expect(untranslated()).toEqual([]);
    });

    it("translates the general tab, what the system asks and an import report", async () => {
      setup();
      await screen.findByRole("option", { name: "Firefox" });
      await tab("settings.tab.general");
      await fireEvent.click(screen.getByRole("button", { name: t("general.default.make") }));
      await fireEvent.input(screen.getByPlaceholderText(/Browsers/), { target: { value: "{}" } });
      await fireEvent.click(screen.getByRole("button", { name: t("general.hurl.import") }));
      await screen.findByText(t("rust.register.confirm_windows"));
      await screen.findByText(t("rust.hurl.uwp", { browser: "Firefox" }));
      await screen.findByText(t("general.hurl.done", { browsers: t("general.hurl.browsers", { count: 1 }), rulesets: t("general.hurl.rulesets", { count: 2 }) }));
      expect(untranslated()).toEqual([]);
    });

    it("translates the about tab and the footer after a save", async () => {
      setup();
      await screen.findByRole("option", { name: "Firefox" });
      await tab("settings.tab.about");
      await fireEvent.click(await screen.findByRole("button", { name: t("about.check") }));
      await screen.findByRole("button", { name: t("about.download") });
      expect(untranslated()).toEqual([]);
      await tab("settings.tab.general");
      await fireEvent.click(screen.getByLabelText(t("general.checkForUpdates")));
      expect(untranslated()).toEqual([]);
      await fireEvent.click(screen.getByRole("button", { name: t("settings.save") }));
      await screen.findByText(t("settings.saved"));
      expect(untranslated()).toEqual([]);
    });
  });

  it("catches a text written in a component", () => {
    document.body.innerHTML = `<p>${t("picker.cancel")}</p><p>Cancel</p><button aria-label="Close"></button><span translate="no">Reroute</span>`;
    expect(untranslated()).toEqual(["text: Cancel", "aria-label: Close"]);
  });
});
