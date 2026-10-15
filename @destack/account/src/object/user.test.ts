import { Expression } from "@destack/db";
import { expect, test } from "@destack/test";
import { user } from "./user.ts";

test("fill an empty name from the email's local part, keeping a given name and a replica's without the email", () => {
    // upgrade rows an earlier release wrote
    const rows = [
        { name: "", email: "ada@example.com" },
        { name: "Ada Lovelace", email: "ada@example.com" },
        { name: "", email: null },
    ];
    const upgraded = rows.map((row) =>
        Expression.upgrade(user.lifecycle.convert, row, "2026.8.0", user.package.version),
    );

    expect(upgraded).toEqual([
        { name: "ada", email: "ada@example.com" },
        { name: "Ada Lovelace", email: "ada@example.com" },
        { name: "", email: null },
    ]);
});
