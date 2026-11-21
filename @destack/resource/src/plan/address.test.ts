import { expect, test } from "@destack/test";
import { Address } from "./address.ts";

test("split an address into the segments it was joined from", () => {
    const address = Address.join("table", "task", "column", "name");

    expect([address, Address.split(address)]).toEqual([
        "table/task/column/name",
        ["table", "task", "column", "name"],
    ]);
});
