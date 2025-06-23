import { uuidt } from "@destack/utils/uuidt";
import { describe, expect, test } from "vitest";

describe("uuidt", () => {
  test("uuidt", () => {
    for (let i = 0; i < 10; i++) {
      const id = uuidt();
      // ensure uuidt is a valid(ish) UUID (should have right form)
      expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    }
  });
});
