import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { Catalog, Localization, t } from "@destack/locale";
import { LocaleContext } from "@destack/view";
import { expect, test } from "@destack/test";
import { Spinner } from "./index.ts";
import { markup, render } from "@destack/view/test";

/** The German catalog this package ships of its built-in messages. */
const GERMAN = Catalog.of(
    "locale/de.json",
    JSON.parse(await readFile(join(import.meta.dirname, "../../locale/de.json"), "utf8")),
    t`Loading`.package,
);

test("announce loading as a status around a hidden icon", () => {
    const { container } = render(() => <Spinner />);
    expect(markup(container)).toBe(
        '<span data-slot="spinner" role="status" aria-label="Loading">' +
            '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg></span>',
    );
});

test("announce loading in the German translation the package ships, for an Austrian reader", () => {
    const { container } = render(() => (
        <LocaleContext value={Localization.of("de-AT", [GERMAN])}>
            <Spinner />
        </LocaleContext>
    ));
    expect(container.firstElementChild?.getAttribute("aria-label")).toBe("Wird geladen");
});
