import { mockWindows } from "@tauri-apps/api/mocks";
import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { mockBackend } from "../app/testBackend";
import { ResizeEdges } from "./resize-edges";

describe("ResizeEdges", () => {
  it("starts a native resize from the edge or corner pressed", async () => {
    mockWindows("main");
    const { calls } = mockBackend({ "plugin:window|start_resize_dragging": () => null });
    const { container } = render(<ResizeEdges />);
    const edges = Array.from(container.querySelectorAll<HTMLElement>("[data-edge]"));
    expect(edges.map((edge) => edge.dataset.edge)).toEqual([
      "North",
      "South",
      "West",
      "East",
      "NorthWest",
      "NorthEast",
      "SouthWest",
      "SouthEast",
    ]);

    const edge = (direction: string) => {
      const found = edges.find((element) => element.dataset.edge === direction);
      if (!found) {
        throw new Error(`no ${direction} edge`);
      }
      return found;
    };
    fireEvent.pointerDown(edge("NorthWest"), { button: 0 });
    fireEvent.pointerDown(edge("East"), { button: 2 });
    await Promise.resolve();

    expect(calls).toEqual([
      {
        cmd: "plugin:window|start_resize_dragging",
        args: { label: "main", value: "NorthWest" },
      },
    ]);
  });
});
