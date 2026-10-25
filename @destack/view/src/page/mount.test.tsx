import { BrowserSimulation } from "@destack/environment/browser/simulation";
import type { BrowserEnvironment } from "@destack/environment/browser";
import { Simulation } from "@destack/environment/simulation";
import { type LocaleTag, Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import type { ViewContext } from "../declare/context.ts";
import { renderView } from "./mount.ts";
import { mountPackages, useNow, usePackage } from "./view.ts";
import { definePackage } from "@destack/object";
import { defineObject, method } from "@destack/object";
import { none } from "@destack/access";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { createSignal } from "solid-js";
import { Errored } from "../solid/flow.ts";
import { useLocale } from "./locale.ts";
import type { Catalog } from "@destack/locale";
import { changed } from "../test/dom.ts";

/** A page origin that calls nothing, as a test page has none. */
function noOrigin(path: string): never {
    throw new TypeError(`a test page reaches no ${path}`);
}

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The longest a test waits for a render, far above the few microtasks a local resource takes. */
const RENDER_MILLISECONDS = 2000;

/** The first strong isolate MessageFormat 2 places around a string value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a string value. */
const CLOSE = String.fromCodePoint(0x2069);

/** What a host gives a view mounted in a test, before setting the person's locale. */
const CONTEXT: ViewContext = {
    installation: "installation-notes",
    space: "space-personal",
    account: "account-florian",
    view: "notes",
    user: principal.user.reference(Scope.universe.id, "user-01996ab0-0000-7000-8000-000000000001"),
};

/** The message the status shows. */
const SAVED = t`Saved ${"3"} notes`;

/** The catalogs of this package: its German translation of the status. */
const CATALOGS: readonly Catalog[] = [
    { package: SAVED.package, locale: "de", messages: { [SAVED.id]: "{$p0} Notizen gespeichert" } },
];

/** Show the mounted view's locale and one message rendered in it. */
function Status() {
    const locale = useLocale();

    return <p>{`${locale.tag}: ${locale.render(SAVED)}`}</p>;
}

/** Render the status into a detached element in a language and read its text. */
function readText(locale: LocaleTag, environment: BrowserEnvironment): string {
    const element = document.createElement("div");
    const dispose = renderView(
        Status,
        element,
        {
            context: CONTEXT,
            clients: new Map(),
            packages: new Map(),
            origin: noOrigin,
        },
        Localization.of(locale, CATALOGS),
        environment,
    );
    const text = element.textContent;
    dispose();

    return text;
}

test("render a view under its localization, translated by its package's catalog and in the source language without one", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = new BrowserSimulation(simulation).environment("view");

    expect([readText(SOURCE_LOCALE, environment), readText("de-AT", environment)]).toEqual([
        `en: Saved ${OPEN}3${CLOSE} notes`,
        `de-AT: ${OPEN}3${CLOSE} Notizen gespeichert`,
    ]);
});

/** The package answering how many notes a person keeps, which declares the tallies. */
const COUNTER = Package.parse({
    id: "package-019f7480-0000-7000-8000-000000000002",
    name: "@example/counter",
    version: "2026.9.0",
});

/** Counts of a person's notes, answered by a package's query. */
const tally = defineObject(
    {
        name: "tally",
        plural: "tallies",
        scope: Scope.universe.id,
        fields: {},
        permissions: { read: none() },
    },
    () => ({
        count: method.query({
            permission: "read",
            target: "scope",
            output: schema.object({ notes: schema.number() }),
        }),
    }),
    { package: COUNTER },
);

/** A package answering how many notes a person keeps. */
const counter = definePackage({ objects: { tally } }, { package: COUNTER });

/** Show the note count the counter package answers. */
function Count() {
    const [notes, setNotes] = createSignal<number>();
    void usePackage(counter)
        .tally.count({})
        .then((answer) => setNotes(answer.notes));

    return <p>{`notes: ${notes() ?? "…"}`}</p>;
}

test("call a package the view declares where the host mounts it, and refuse it to a view declaring none", async () => {
    // mount the counter at the view's platform path, answering from a fetch recording the path
    await using simulation = new Simulation({ seed: 1, start: START });
    const paths: string[] = [];
    const mounted = mountPackages([counter], (path) => ({
        url: new URL(path, "https://notes--personal--florian.localhost").href,
    }));
    const packages = new Map(
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
    const dispose = renderView(
        Count,
        element,
        {
            context: CONTEXT,
            clients: new Map(),
            packages,
            origin: noOrigin,
        },
        Localization.of(SOURCE_LOCALE, []),
        new BrowserSimulation(simulation).environment("view"),
    );

    // read the count through the declared package's mount
    await changed(element, () => element.textContent === "notes: 3", RENDER_MILLISECONDS);
    dispose();
    expect(paths).toEqual([`/.destack/platform/${counter.id}/tallies/count`]);

    // refuse the package to a view declaring none, into the error boundary
    const refused = document.createElement("div");
    const disposeRefused = renderView(
        () => (
            <Errored fallback={(error) => <p>{String(error())}</p>}>
                {(() => {
                    usePackage(counter);

                    return <p>counted</p>;
                })()}
            </Errored>
        ),
        refused,
        {
            context: CONTEXT,
            clients: new Map(),
            packages: new Map(),
            origin: noOrigin,
        },
        Localization.of(SOURCE_LOCALE, []),
        new BrowserSimulation(simulation).environment("view"),
    );
    expect(refused.textContent).toBe(
        "FamilyError: the view does not declare package @example/counter",
    );
    disposeRefused();
});

/** Show the time on the mounted view's clock. */
function Now() {
    const now = useNow();

    return <p>{String(now())}</p>;
}

test("show the time on the view's clock, moving on each second until the view goes", async () => {
    // render the time on a simulated clock
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock } = simulation;
    const start = clock.now();
    const element = document.createElement("div");
    const dispose = renderView(
        Now,
        element,
        {
            context: CONTEXT,
            clients: new Map(),
            packages: new Map(),
            origin: noOrigin,
        },
        Localization.of(SOURCE_LOCALE, []),
        new BrowserSimulation(simulation).environment("view"),
    );
    const first = element.textContent;

    // move the clock a second, then two more once the view went
    clock.advance(1000);
    await changed(element, () => element.textContent !== first, RENDER_MILLISECONDS);
    const moved = element.textContent;
    dispose();
    clock.advance(2000);
    expect([first, moved, clock.next]).toEqual([String(start), String(start + 1000), undefined]);
});
