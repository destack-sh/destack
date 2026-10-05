import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { Catalog, t } from "@destack/locale";
import { expect, onTestFinished, test } from "@destack/test";
import { Loading, type JSX } from "@destack/view";
import { renderView } from "@destack/view/test";
import { NotificationBadge, NotificationInbox } from "../../src/view/index.ts";
import { openHome, type Home } from "./home.ts";

/** The German catalog this package ships of its built-in messages. */
const GERMAN = Catalog.of(
    "locale/de.json",
    JSON.parse(await readFile(join(import.meta.dirname, "../../locale/de.json"), "utf8")),
    t`Unread`.package,
);

/** Render an element as a mounted view whose host opens bob's home in a locale, removing it after the test. */
function drawInHome(
    home: Home,
    element: () => JSX.Element,
    catalogs: readonly Catalog[] = [],
): HTMLElement {
    const container = document.createElement("main");
    document.body.append(container);
    onTestFinished(() => container.remove());
    onTestFinished(
        renderView(
            () => <Loading>{element()}</Loading>,
            container,
            home.context,
            home.clients,
            catalogs,
            new Map(),
        ),
    );

    return container;
}

/** Read each notification row's state and line, leaving out its relative time. */
function rows(container: Element): string[] {
    return [...container.querySelectorAll("[data-slot=notification]")].map((row) => {
        const time = row.querySelector("time")?.textContent ?? "";

        return `${row.getAttribute("data-state")}: ${row.querySelector("button")?.textContent?.replace(time, "")}`;
    });
}

test("list the person's notifications from their home and mark one read when opened", async () => {
    const home = await openHome();
    const container = drawInHome(home, () => <NotificationInbox />);
    await expect
        .poll(() => rows(container))
        .toEqual(["unread: Unread 1 mention", "unread: Unread 1 mention"]);

    // opening the first predicts it read, without failures from the server
    container.querySelector<HTMLElement>("[data-slot=notification] button")?.click();
    await expect
        .poll(() => rows(container))
        .toEqual(["read: 1 mention", "unread: Unread 1 mention"]);
    expect(home.failures).toEqual([]);
});

test("count the person's unread notifications in every space, or in one", async () => {
    const home = await openHome();
    const container = drawInHome(home, () => (
        <>
            <NotificationBadge />
            <NotificationBadge space={home.context.space} />
            <NotificationBadge space="space-019f5530-8000-7000-8000-0000000000ff" />
        </>
    ));

    // both unread notifications happened in the view's space, and none elsewhere
    await expect
        .poll(() =>
            [...container.querySelectorAll("[data-slot=notification-badge]")].map((badge) => [
                badge.querySelector("[aria-hidden=true]")?.textContent,
                badge.querySelector(":not([aria-hidden])")?.textContent,
            ]),
        )
        .toEqual([
            ["2", "2 unread notifications"],
            ["2", "2 unread notifications"],
        ]);
});

test("count unread notifications and mark them unread in the German drafts the package ships", async () => {
    const home = await openHome();
    const german = { ...home, context: { ...home.context, locale: "de-AT" } };
    const container = drawInHome(
        german,
        () => (
            <>
                <NotificationBadge />
                <NotificationInbox />
            </>
        ),
        [GERMAN],
    );

    // the spoken count and each row's hidden state read in German, the fixture's own summary as written
    await expect
        .poll(() => [
            container.querySelector("[data-slot=notification-badge] :not([aria-hidden])")
                ?.textContent,
            ...rows(container),
        ])
        .toEqual([
            "2 ungelesene Benachrichtigungen",
            "unread: Ungelesen 1 mention",
            "unread: Ungelesen 1 mention",
        ]);
});
