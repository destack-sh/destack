import { expect, test } from "@destack/test";
import { Locale } from "./locale.ts";
import { Localization } from "../localization/localization.ts";

test("read language tags in their canonical casing and refuse malformed ones", () => {
    expect([Locale.parse("de-at"), Locale.parse("ZH-hant-tw"), Locale.parse("es-419")]).toEqual([
        "de-AT",
        "zh-Hant-TW",
        "es-419",
    ]);
    expect(() => Locale.parse("not a tag")).toThrow(new TypeError("not a language tag: not a tag"));
});

test("walk a tag's fallback chain through CLDR's parents, else its shorter tags, to the source language once", () => {
    expect([
        Locale.fallback("de-AT", "en"),
        Locale.fallback("en-AU", "en"),
        Locale.fallback("en-AT", "en"),
        Locale.fallback("es-MX", "en"),
        Locale.fallback("zh-TW", "en"),
        Locale.fallback("en-US", "en"),
        Locale.fallback("en", "en"),
    ]).toEqual([
        ["de-AT", "de", "en"],
        ["en-AU", "en-001", "en"],
        ["en-AT", "en-150", "en-001", "en"],
        ["es-MX", "es-419", "es", "en"],
        ["zh-TW", "zh-Hant", "zh", "en"],
        ["en-US", "en"],
        ["en"],
    ]);
});

test("pick the available tag serving a person's preferences best, in their order, absent without a shared language", () => {
    expect([
        Locale.negotiate(["de-AT", "en"], ["fr", "de"]),
        Locale.negotiate(["pt-BR", "de"], ["de", "pt"]),
        Locale.negotiate(["es-MX"], ["es-419", "es"]),
        Locale.negotiate(["ja"], ["fr", "de"]),
    ]).toEqual(["de", "pt", "es-419", undefined]);
});

test("read the tags an Accept-Language header asks for by weight, leaving out the wildcard, refused weights and malformed tags", () => {
    expect([
        Locale.accepted("fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5"),
        Locale.accepted("en;q=0.5, de-at, x y, es;q=0"),
        Locale.accepted(""),
    ]).toEqual([["fr-CH", "fr", "en", "de"], ["de-AT", "en"], []]);
});

test("read a locale's direction from the script it most likely writes in, unless a localization sets another", () => {
    expect([
        ...["ar-EG", "he", "fa", "de-AT", "sr-Cyrl", "zh-Hant"].map((tag) => Locale.direction(tag)),
        Localization.of("ar", []).direction,
        Localization.of("en", [], { direction: "rtl" }).direction,
    ]).toEqual(["rtl", "rtl", "rtl", "ltr", "ltr", "ltr", "rtl", "rtl"]);
});
