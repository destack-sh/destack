import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import { RequestId } from "@destack/service/request";
import type { Server } from "@destack/service/server";
import { expect, onTestFinished, test } from "@destack/test";
import { Loading, type JSX } from "@destack/view";
import { renderView } from "@destack/view/test";
import { eventService } from "@destack/event/service";
import { alert, alertRule, issue } from "../../src/object/index.ts";
import { observabilityService } from "../../src/service/index.ts";
import Alerts from "../../src/view/alert.tsx";
import Issues from "../../src/view/issue.tsx";
import Logs from "../../src/view/log.tsx";
import Traces from "../../src/view/trace.tsx";
import { client, ids, serveObservability } from "../fixture/observability.ts";
import { logsExport, NOTES_EMITTER, tracesExport } from "../fixture/notes.ts";

/** Reach a server's service as alice. */
function reach(server: Server) {
    return {
        url: "https://observability.test",
        headers: { authorization: "Bearer alice" },
        fetch: (request: Request) => server.fetch(request),
    };
}

/** Open alice's device following the space's issues, alert rules and alerts, closed after the test. */
async function openDevice(server: Server): Promise<ObjectClient> {
    // keep the objects in a local replica database
    const objects = { issue, alertRule, alert };
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables(objects), {
        isMigrated: true,
        isReplica: true,
    });
    const device = await ObjectClient.open({
        database: storage.database,
        objects,
        package: observabilityService.package,
        scope: ids.space,
        caller: principal.user.reference("universe", "alice"),
        endpoint: reach(server),
        reconnect: () => reach(server),
    });

    // follow and push until the test ends, failing it on any reported failure
    const following = new AbortController();
    const failures: unknown[] = [];
    const report = (error: unknown) => failures.push(error);
    const loops = [device.follow(following.signal, report), device.push(following.signal, report)];
    onTestFinished(async () => {
        following.abort();
        await Promise.all(loops);
        await device.close();
        await storage.close();
        expect(failures).toEqual([]);
    });

    return device;
}

/** Render a page as a view of the space alice owns on her device, calling the observability and event services as her, removing it after the test. */
async function draw(
    served: { readonly server: Server; readonly eventServer: Server },
    view: string,
    page: () => JSX.Element,
): Promise<{ readonly container: HTMLElement; readonly device: ObjectClient }> {
    const device = await openDevice(served.server);
    const container = document.createElement("main");
    document.body.append(container);
    onTestFinished(() => container.remove());
    onTestFinished(
        renderView(
            () => <Loading>{page()}</Loading>,
            container,
            {
                space: ids.space,
                account: ids.account,
                view,
                user: principal.user.reference("universe", "alice"),
            },
            { [ids.space]: device },
            [],
            new Map([
                [observabilityService.package.id, reach(served.server)],
                [eventService.package.id, reach(served.eventServer)],
            ]),
        ),
    );

    return { container, device };
}

/** Read the queued mutations of a device by outcome. */
async function mutations(device: ObjectClient) {
    return (await device.inspect()).mutations;
}

/** Read the cells of each row a selector finds, leaving out the first, the time each row shows in the local zone. */
function rows(container: Element, selector: string): string[][] {
    return [...container.querySelectorAll(selector)].map((row) =>
        [...row.querySelectorAll("td")].slice(1).map((cell) => cell.textContent ?? ""),
    );
}

/** Read the text of each element a selector finds. */
function texts(container: Element, selector: string): string[] {
    return [...container.querySelectorAll(selector)].map((found) => found.textContent ?? "");
}

/** Type a filter into a page's filter field. */
function type(container: Element, filter: string): void {
    const input = container.querySelector<HTMLInputElement>("input[aria-label=Filter]");
    if (input === null) {
        throw new TypeError("the page shows no filter field");
    }
    input.value = filter;
    input.dispatchEvent(new Event("input", { bubbles: true }));
}

test("list a space's recent log records newest first, narrow them with a typed filter, and follow new ones live", async () => {
    // record an informational and an error record a millisecond apart
    const served = await serveObservability();
    const { receive } = served;
    const now = Date.now();
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([
            { time: now, name: "note.opened", severity: 9, body: "opened Plans" },
            { time: now + 1, name: "note.failed", severity: 17, body: "disk full" },
        ]),
    );
    const { container } = await draw(served, "logs", () => <Logs />);
    await expect.poll(() => rows(container, "[data-slot=log]")).toHaveLength(2);
    const listed = rows(container, "[data-slot=log]");

    // narrow to errors, then record another one
    type(container, "severity >= 17");
    await expect.poll(() => rows(container, "[data-slot=log]")).toHaveLength(1);
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([{ time: now + 2, name: "note.lost", severity: 18, body: "gone" }]),
    );

    // show both records newest first, then the errors, the new one first once it arrives
    await expect.poll(() => rows(container, "[data-slot=log]"), { timeout: 4000 }).toHaveLength(2);
    expect({ listed, followed: rows(container, "[data-slot=log]") }).toEqual({
        listed: [
            ["error", "note.failed", "disk full"],
            ["info", "note.opened", "opened Plans"],
        ],
        followed: [
            ["error", "note.lost", "gone"],
            ["error", "note.failed", "disk full"],
        ],
    });
});

test("list a space's recent traces and show the open one's spans as a tree on its time line", async () => {
    // record a request span with a child that failed
    const served = await serveObservability();
    const { receive } = served;
    const now = Date.now();
    const trace = `${now.toString(16).padStart(12, "0")}${"4".repeat(20)}`;
    await receive(
        NOTES_EMITTER,
        "traces",
        tracesExport([
            {
                trace,
                span: "00f067aa0ba902b7",
                name: "GET /notes",
                start: now,
                end: now + 4,
                status: 1,
            },
            {
                trace,
                span: "00f067aa0ba902b8",
                parent: "00f067aa0ba902b7",
                name: "note.load",
                start: now + 1,
                end: now + 3,
                status: 2,
            },
        ]),
    );

    // open the trace from the list
    const { container } = await draw(served, "traces", () => <Traces />);
    await expect
        .poll(() => texts(container, "[data-slot=trace] [data-slot=item-title]"))
        .toHaveLength(1);
    container.querySelector<HTMLElement>("[data-slot=trace]")?.click();
    await expect.poll(() => texts(container, "[data-slot=span]")).toHaveLength(2);

    // list the root first, its child indented below it
    expect({
        traces: texts(container, "[data-slot=trace] [data-slot=item-title]"),
        spans: texts(container, "[data-slot=span]"),
        depths: [...container.querySelectorAll("[data-slot=span]")].map((span) =>
            span.getAttribute("data-depth"),
        ),
    }).toEqual({
        traces: ["GET /notesok"],
        spans: ["GET /notes · 4 ms", "note.load · 2 ms"],
        depths: ["0", "1"],
    });
});

test("resolve an issue a failure opened from its page, which lists its events", async () => {
    // fail once in the notes installation, opening an issue with one event
    const served = await serveObservability();
    const { receive } = served;
    const now = Date.now();
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([
            {
                time: now,
                name: "exception",
                severity: 17,
                attributes: { "exception.message": "title is empty", "error.type": "TypeError" },
            },
        ]),
    );

    // open it from the list and resolve it
    const { container, device } = await draw(served, "issues", () => <Issues />);
    await expect.poll(() => texts(container, "[data-slot=issue]")).toHaveLength(1);
    container.querySelector<HTMLElement>("[data-slot=issue]")?.click();
    await expect.poll(() => texts(container, "[data-slot=event]")).toHaveLength(1);
    const events = texts(container, "[data-slot=event] [data-slot=item-title]");
    [...container.querySelectorAll<HTMLElement>("button")]
        .find((button) => button.textContent === "Resolve")
        ?.click();

    // show its event, then its resolution, which the server confirms
    await expect.poll(() => texts(container, "[data-slot=status]")).toEqual(["resolved"]);
    await expect.poll(() => mutations(device)).toEqual({ pending: 0, executed: 0, rejected: 0 });
    expect(events).toEqual(["title is empty"]);
});

test("list a space's alert rules beside its alerts, and resolve a firing alert by hand", async () => {
    // log an error, then keep a rule firing on any error log
    const served = await serveObservability();
    const { server, receive } = served;
    const alice = client(server, "alice");
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([{ time: Date.now(), name: "note.failed", severity: 17 }]),
    );
    await alice.alertRule.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        name: "Errors",
        condition: {
            kind: "events",
            event: "log",
            where: "severity >= 17",
            fold: "count",
            group: [],
            window: { minutes: 5 },
            comparison: "above",
            threshold: 0,
        },
        actions: [{ kind: "notify" }],
    });

    // resolve the alert from the page
    const { container, device } = await draw(served, "alerts", () => <Alerts />);
    await expect
        .poll(() => texts(container, "[data-slot=alert] [data-slot=item-title]"))
        .toHaveLength(1);
    container.querySelector<HTMLElement>("[data-slot=alert] button")?.click();

    // show the rule and the alert, resolved, which the server confirms
    await expect
        .poll(() => texts(container, "[data-slot=alert] [data-slot=item-title]"))
        .toEqual(["log count 1resolved"]);
    await expect.poll(() => mutations(device)).toEqual({ pending: 0, executed: 0, rejected: 0 });
    expect(texts(container, "[data-slot=rule] [data-slot=item-title]")).toEqual(["Errors"]);
});
