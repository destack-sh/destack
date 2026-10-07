import { t } from "@destack/locale";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import type { ViewContext } from "../declare/context.ts";
import { renderView } from "./mount.ts";
import { mountServices, useService } from "./view.ts";
import { defineProcedure, defineService } from "@destack/service";
import { schema } from "@destack/schema";
import { createSignal } from "solid-js";
import { Errored } from "../solid/flow.ts";
import { useLocale } from "./locale.ts";
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
    const dispose = renderView(Status, element, context, {}, CATALOGS, new Map());
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

/** A platform service answering how many notes a person keeps. */
const counter = defineService("counter", {
    count: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/count" })
        .output(schema.object({ notes: schema.number() })),
});

/** Show the note count the counter service answers. */
function Count() {
    const [notes, setNotes] = createSignal<number>();
    void useService(counter)
        .count()
        .then((answer) => setNotes(answer.notes));

    return <p>{`notes: ${notes() ?? "…"}`}</p>;
}

test("call a platform service the view declares where the host mounts it, and refuse it to a view declaring none", async () => {
    // mount the counter at the view's platform path, answering from a fetch recording the path
    const paths: string[] = [];
    const mounted = mountServices([counter], "https://notes.personal.florian.localhost");
    const services = new Map(
        [...mounted].map(([packageId, options]) => [
            packageId,
            {
                ...options,
                fetch: async (request: Request) => {
                    paths.push(new URL(request.url).pathname);

                    return Response.json({ notes: 3 });
                },
            },
        ]),
    );
    const element = document.createElement("div");
    const dispose = renderView(Count, element, CONTEXT, {}, [], services);

    // read the count through the declared service's mount
    await expect.poll(() => element.textContent).toBe("notes: 3");
    dispose();
    expect(paths).toEqual([`/.destack/platform/service/${counter.package.id}/count`]);

    // refuse the service to a view declaring none, into the error boundary
    const refused = document.createElement("div");
    const disposeRefused = renderView(
        () => (
            <Errored fallback={(error) => <p>{String(error())}</p>}>
                {(() => {
                    useService(counter);

                    return <p>counted</p>;
                })()}
            </Errored>
        ),
        refused,
        CONTEXT,
        {},
        [],
        new Map(),
    );
    expect(refused.textContent).toBe("TypeError: the view declares no counter service");
    disposeRefused();
});
