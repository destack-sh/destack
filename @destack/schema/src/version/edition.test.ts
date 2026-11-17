import { expect, test } from "@destack/test";
import { Edition } from "./edition.ts";

/** Check conversion keys, reading a refusal as its message. */
function refusal(keyed: Record<string, string>): string {
    try {
        Edition.require(keyed, "user");
    } catch (error) {
        return String(error);
    }

    return "accepted";
}

test("read the edition conversions lead to, the first without any", () => {
    expect([Edition.of(), Edition.of({}), Edition.of({ 2: "a" }, { 4: "b", 3: "c" })]).toEqual([
        1, 1, 4,
    ]);
});

test("list the conversions after one edition and up to another in order", () => {
    const keyed = { 4: "d", 2: "b", 3: "c" };
    expect([
        Edition.between(keyed, 1, 4),
        Edition.between(keyed, 2, 3),
        Edition.between(keyed, 4, 4),
        Edition.between(keyed, 1, 1),
    ]).toEqual([
        [
            [2, "b"],
            [3, "c"],
            [4, "d"],
        ],
        [[3, "c"]],
        [],
        [],
    ]);
});

test("refuse conversion keys that are no edition after the first", () => {
    // accept editions from the second on
    Edition.require({ 2: "b", 7: "g" }, "user");

    // refuse the first edition, a fraction and a release
    expect([refusal({ 1: "a" }), refusal({ "2.5": "b" }), refusal({ "2026.11.0": "c" })]).toEqual([
        "TypeError: conversion key of 'user' is no edition after 1: '1'",
        "TypeError: conversion key of 'user' is no edition after 1: '2.5'",
        "TypeError: conversion key of 'user' is no edition after 1: '2026.11.0'",
    ]);
});
