import { t } from "@destack/locale";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import type { ViewContext } from "../declare/context.ts";
import { renderView } from "./mount.ts";
import { useLocale } from "@destack/locale/solid";
import type { Catalog } from "@destack/locale";

/** The first strong isolate MessageFormat 2 places around a string value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a string value. */
const CLOSE = String.fromCodePoint(0x2069);

/** What a host gives a view mounted in a test, before naming the person's locale. */
const CONTEXT: ViewContext = {
    installation: "installation-notes",
    space: "space-personal",
    account: "account-florian",
    view: "notes",
    user: principal.user.reference(Scope.universe.id, "user-florian"),
};

/** The message the status shows. */
const SAVED = t`Saved ${"3"} notes`;

/** The launch's catalogs: this package's German translation of the status. */
const CATALOGS: readonly Catalog[] = [
    { package: SAVED.package, locale: "de", messages: { [SAVED.id]: "{$p0} Notizen gespeichert" } },
];

/** Show the mounted view's locale and one message rendered in it. */
function Status() {
    const locale = useLocale();

    return <p>{`${locale.tag}: ${locale.render(SAVED)}`}</p>;
}

/** Render the status into a detached element and read its text. */
function textOf(context: ViewContext): string {
    const element = document.createElement("div");
    const dispose = renderView(Status, element, context, {}, CATALOGS);
    const text = element.textContent;
    dispose();

    return text;
}

test("render in the person's locale from the view's context with the launch's catalogs, the source language when the host names none", () => {
    expect([textOf(CONTEXT), textOf({ ...CONTEXT, locale: "de-AT" })]).toEqual([
        `en: Saved ${OPEN}3${CLOSE} notes`,
        `de-AT: ${OPEN}3${CLOSE} Notizen gespeichert`,
    ]);
});
