import { expect, test } from "@destack/test";
import { createPermission } from "./permission.ts";

test("know no permission on the server", () => {
    expect(createPermission("camera")()).toBe("unknown");
});
