import { expect, test } from "@destack/test";
import { AccountHandle } from "./handle.ts";

test("make a handle stem from a name, falling back to the default for a name without handle characters", () => {
    expect([
        AccountHandle.stem("Ada.Lovelace"),
        AccountHandle.stem("--ada--"),
        AccountHandle.stem("ünïcödé"),
        AccountHandle.stem("a-very-long-login-name-that-goes-on"),
    ]).toEqual(["ada-lovelace", "ada", "n-c-d", "a-very-long-login-name-t"]);
});

test("number the candidates past the first, across a batch boundary, trimmed to the handle length", () => {
    expect([
        AccountHandle.candidates("crowded", 1, 3),
        AccountHandle.candidates("crowded", 99, 3),
        AccountHandle.candidates("a-very-long-login-name-t", 100, 1),
    ]).toEqual([
        ["crowded", "crowded-2", "crowded-3"],
        ["crowded-99", "crowded-100", "crowded-101"],
        ["a-very-long-login-na-100"],
    ]);
});
