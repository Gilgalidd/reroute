// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Picker from "../routes/Picker.svelte";
import Settings from "../routes/Settings.svelte";
import { describeError } from "../lib/api";
import { launchContext, mockIpc, rustMessage, sparseConfig } from "../test/ipc";
import { LANGUAGES, language, languageName, setLanguage, tRust } from "./i18n.svelte";

describe("the language setting", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  afterEach(() => {
    setLanguage("en");
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
  });

  it("knows English and French, each by its own name", () => {
    expect(LANGUAGES).toEqual(["en", "fr"]);
    expect(languageName("fr")).toBe("Français");
  });

  it("follows the system in auto", () => {
    vi.stubGlobal("navigator", { languages: ["fr-CA", "en"] });
    setLanguage("auto");
    expect(language()).toBe("fr");
    expect(document.documentElement.lang).toBe("fr");
  });

  it("takes the pseudo-language for auto in a release", () => {
    vi.stubEnv("DEV", false);
    vi.stubGlobal("navigator", { languages: ["en-GB"] });
    setLanguage("pseudo");
    expect(language()).toBe("en");
  });

  it("draws the picker in the language it is given", async () => {
    const base = launchContext();
    mockIpc({ launch_context: () => ({ ...base, settings: { ...base.settings, language: "fr" } }) });
    render(Picker);
    expect(await screen.findByRole("button", { name: "Annuler" })).toBeInTheDocument();
    expect(screen.getByText("Toujours utiliser pour", { exact: false })).toBeInTheDocument();
  });

  it("switches the settings window at once, its title too, and saves the choice", async () => {
    const ipc = mockIpc({
      get_config: () => ({ config: sparseConfig(), load_error: null }),
      default_browser_status: () => true,
    });
    render(Settings);
    await fireEvent.click(await screen.findByRole("button", { name: "General" }));
    expect(document.title).toBe("Reroute settings");
    await fireEvent.change(screen.getByLabelText("Language"), { target: { value: "fr" } });
    expect(await screen.findByRole("button", { name: "Enregistrer" })).toBeInTheDocument();
    expect(document.title).toBe("Paramètres de Reroute");
    await fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(ipc.saved).toHaveLength(1));
    expect(ipc.saved[0]?.settings.language).toBe("fr");
  });

  it("goes back to the saved language on Discard", async () => {
    mockIpc({
      get_config: () => ({ config: sparseConfig(), load_error: null }),
      default_browser_status: () => true,
    });
    render(Settings);
    await fireEvent.click(await screen.findByRole("button", { name: "General" }));
    await fireEvent.change(screen.getByLabelText("Language"), { target: { value: "fr" } });
    await fireEvent.click(await screen.findByRole("button", { name: "Abandonner" }));
    expect(await screen.findByRole("button", { name: "Save" })).toBeInTheDocument();
  });
});

describe("messages from Rust", () => {
  afterEach(() => {
    setLanguage("en");
  });

  const regex = rustMessage("pattern.invalid_regex", { detail: "unclosed group" }, "invalid regex: unclosed group");
  const nested = rustMessage("config.pattern", { ruleset: "Work", problem: regex }, "ruleset `Work`: invalid regex: unclosed group");

  it("are worded in the current language, a reason inside a reason too", () => {
    expect(tRust(nested)).toBe("ruleset “Work”: not a valid regular expression: unclosed group");
    setLanguage("fr");
    expect(tRust(nested)).toBe("jeu de règles «\u202fWork\u202f»\u00a0: expression régulière invalide\u00a0: unclosed group");
  });

  it("write numbers the way the language does", () => {
    setLanguage("fr");
    expect(tRust(rustMessage("url.too_long", { bytes: 70000 }))).toBe("le lien est trop long (70\u202f000 octets)");
  });

  it("show Rust's own text for a code the catalogue does not have", () => {
    setLanguage("fr");
    expect(tRust(rustMessage("from.a.newer.build", {}, "something new happened"))).toBe("something new happened");
  });

  it("are what a failed command throws; anything else keeps its text", () => {
    setLanguage("fr");
    expect(describeError(rustMessage("release.timeout"))).toBe("le serveur n’a pas répondu à temps");
    expect(describeError("bridge unavailable")).toBe("bridge unavailable");
    expect(describeError(new Error("boom"))).toBe("boom");
  });
});
