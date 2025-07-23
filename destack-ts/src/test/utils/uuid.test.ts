import { test } from "bun:test";
import { uuid7 } from "@destack/utils/uuid";

test("generate uuid7", () => {
  for (let i = 0; i < 10; i++) {
    const uuid = uuid7();
  }
});
