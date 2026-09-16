import { describe, expect, it } from "vitest";
import { CURSEFORGE_SORTS, MODRINTH_SORTS, defaultSortForSource, sortsForSource } from "./contentSort";

describe("sortsForSource", () => {
  it("returns CurseForge's own sort fields for curseforge", () => {
    expect(sortsForSource("curseforge")).toBe(CURSEFORGE_SORTS);
  });

  it("defaults to Modrinth's sort fields for any other source", () => {
    expect(sortsForSource("modrinth")).toBe(MODRINTH_SORTS);
    expect(sortsForSource(undefined)).toBe(MODRINTH_SORTS);
  });
});

describe("defaultSortForSource", () => {
  it("picks each provider's first real sort option, not a made-up default", () => {
    expect(defaultSortForSource("modrinth")).toBe("relevance");
    expect(defaultSortForSource("curseforge")).toBe("featured");
  });
});
