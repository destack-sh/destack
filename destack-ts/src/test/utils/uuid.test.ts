import { uuid7 } from "@destack/utils/uuid";
import { test } from "vitest";

test("generate uuid7", () => {
  for (let i = 0; i < 10; i++) {
    const uuid = uuid7();
    console.log(uuid);
  }
});
