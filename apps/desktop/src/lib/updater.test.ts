import { describe, expect, it } from "vitest";
import { updateMetadata } from "./updater";

describe("updateMetadata", () => {
  it("extracts the release title and notes", () => {
    const result = updateMetadata("0.1.3-main.42", "# Timeline update\n\n- Fixed Ctrl+B", "2026-09-10T12:00:00Z");
    expect(result).toMatchObject({
      version: "0.1.3-main.42",
      title: "Timeline update",
      notes: "- Fixed Ctrl+B",
      date: "2026-09-10T12:00:00Z",
    });
    expect(result.portableUrl).toContain("/v0.1.3-main.42/Banshee-Video-Editor-0.1.3-main.42-portable.zip");
  });

  it("uses safe fallback text when release notes are empty", () => {
    const result = updateMetadata("0.1.3-main.43");
    expect(result.title).toBe("Banshee 0.1.3-main.43");
    expect(result.notes).toBe("Виправлення та покращення редактора.");
  });
});
