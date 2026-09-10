import { describe, expect, it } from "vitest";
import { formatBytes, formatTime } from "./format";

describe("format helpers", () => {
  it("formats video time", () => expect(formatTime(683.5)).toBe("11:23"));
  it("does not expose negative time", () => expect(formatTime(-1)).toBe("00:00"));
  it("formats media sizes", () => expect(formatBytes(1024 * 1024)).toBe("1.0 МБ"));
});
