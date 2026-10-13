import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { CustomTransform, Settings, SettingsPatch } from "../lib/ipc";
import { TransformsPage } from "./TransformsPage";
import { mockBackend, SETTINGS, SHORTCUTS } from "./testBackend";

type TransformsPatch = NonNullable<SettingsPatch["transforms"]>;

function backend(initial: Settings = SETTINGS) {
  let settings = initial;
  return mockBackend({
    get_settings: () => settings,
    update_settings: (args) => {
      const patch = (args.patch as SettingsPatch).transforms as TransformsPatch;
      const transforms = settings.transforms;
      settings = {
        ...settings,
        transforms: {
          ...transforms,
          ...(patch.enabled === undefined ? {} : { enabled: patch.enabled }),
          ...(patch.custom ? { custom: patch.custom } : {}),
          ...(patch.autoApply === undefined ? {} : { autoApply: patch.autoApply }),
          polish: {
            ...transforms.polish,
            ...(patch.polish?.instructions ? { instructions: patch.polish.instructions } : {}),
            rules: { ...transforms.polish.rules, ...patch.polish?.rules },
          },
        },
      };
      return settings;
    },
    get_shortcuts: () => SHORTCUTS,
  });
}

const enabled: Settings = { ...SETTINGS, transforms: { ...SETTINGS.transforms, enabled: true } };

describe("TransformsPage", () => {
  it("turns Transforms on and off", async () => {
    const { argsOf } = backend();
    render(<TransformsPage />);
    const toggle = await screen.findByRole("switch", { name: "Transforms" });
    expect(screen.getByRole("switch", { name: "Make more concise" })).toBeDisabled();
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { transforms: { enabled: true } } }]),
    );
    await waitFor(() =>
      expect(screen.getByRole("switch", { name: "Make more concise" })).toBeEnabled(),
    );
  });

  it("toggles Polish rules and adds custom instructions with their limits", async () => {
    const { argsOf } = backend(enabled);
    render(<TransformsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "Reorder for readability" }));
    const input = screen.getByRole("textbox", { name: "New instruction" });
    fireEvent.change(input, { target: { value: "Use British spelling" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    await screen.findByText("Use British spelling");
    fireEvent.change(input, { target: { value: "use british spelling" } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("already in the list");
    fireEvent.change(input, { target: { value: "word ".repeat(51) } });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("under 50 words");
    expect(argsOf("update_settings")).toEqual([
      { patch: { transforms: { polish: { rules: { reorder: false } } } } },
      { patch: { transforms: { polish: { instructions: ["Use British spelling"] } } } },
    ]);
  });

  it("saves a new custom transform as it is typed, suggesting Opt+3 first", async () => {
    const { argsOf } = backend(enabled);
    render(<TransformsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "New transform" }));
    const card = screen.getByRole("region", { name: "Transform 1" });
    fireEvent.change(within(card).getByRole("textbox", { name: "Name" }), {
      target: { value: "Pirate" },
    });
    fireEvent.change(within(card).getByRole("textbox", { name: "Prompt" }), {
      target: { value: "Rewrite like a pirate." },
    });
    await waitFor(
      () => {
        const saved = argsOf("update_settings").at(-1)?.patch as SettingsPatch | undefined;
        const custom = saved?.transforms?.custom as CustomTransform[] | undefined;
        expect(custom?.[0]).toMatchObject({
          name: "Pirate",
          prompt: "Rewrite like a pirate.",
          shortcut: "Opt+3",
        });
      },
      { timeout: 3000 },
    );
    expect(await screen.findByRole("region", { name: "Pirate" })).toBeInTheDocument();
  });

  it("chooses a transform to apply after every dictation", async () => {
    const { argsOf } = backend(enabled);
    render(<TransformsPage />);
    fireEvent.change(
      await screen.findByRole("combobox", { name: "Apply a transform after every dictation" }),
      { target: { value: "polish" } },
    );
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { transforms: { autoApply: "polish" } } },
      ]),
    );
  });
});
