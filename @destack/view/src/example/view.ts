import { PermissionScope, principal } from "@destack/access";
import { WasmClient } from "@destack/db/browser";
import { SqliteDatabase } from "@destack/db/sqlite";
import type { ObjectType } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import type { ExampleObjects } from "@destack/package/declare";
import { Scope } from "@destack/sync";
import type { ViewContext } from "../declare/context.ts";
import type { View } from "../declare/view.ts";
import { type Component, type JSX, createComponent } from "../solid/component.ts";
import { Loading } from "../solid/flow.ts";
import { createMemo, onCleanup } from "../solid/reactive.ts";
import { type ViewMount, ViewMountContext } from "../page/view.ts";

/** The sample space a sample view opens in. */
const SAMPLE_SPACE = "space-01996ab0-0000-7000-8000-00000000e001";

/** The sample person's home. */
const SAMPLE_HOME = "space-01996ab0-0000-7000-8000-00000000e002";

/** The account of the sample space. */
const SAMPLE_ACCOUNT = "account-01996ab0-0000-7000-8000-00000000e003";

/** The address of the sample space's services, which never answer. */
const OFFLINE = { url: "https://sample.invalid", fetch: () => new Promise<Response>(() => {}) };

/** What a view's example renders: the view, and the calls bringing its objects into the example's state. */
interface ViewExampleProperties {
    /** The view to render. */
    readonly view: View;
    /** The calls by scope, each the person's own local change. */
    readonly objects: ExampleObjects;
}

/**
 * Render a view's example offline over the objects its calls bring about, each scope in memory.
 *
 * Each scope the view opens keeps its objects in memory, written by the declared calls as the person's local changes.
 */
export function ViewExample(properties: ViewExampleProperties): JSX.Element {
    // open the scopes and load the view's component, closing the scopes on unmount
    const closing = new AbortController();
    onCleanup(() => closing.abort());
    const opened = createMemo(() => openSample(properties, closing.signal));

    return createComponent(Loading, {
        get children() {
            const { component, mount } = opened();

            return createComponent(ViewMountContext, {
                value: mount,
                get children() {
                    return createComponent(component, {});
                },
            });
        },
    });
}

/** Open a view's scopes in memory, make the example's calls in them and load its component. */
async function openSample(
    properties: ViewExampleProperties,
    signal: AbortSignal,
): Promise<{ readonly component: Component; readonly mount: ViewMount }> {
    // name the sample person in the sample space
    const { view, objects } = properties;
    const context: ViewContext = {
        installation: "installation-sample",
        space: SAMPLE_SPACE,
        account: SAMPLE_ACCOUNT,
        home: SAMPLE_HOME,
        view: view.name,
        user: principal.user.reference(Scope.universe.id, "sample"),
    };

    // open each scope the view opens objects in, and make the example's calls there
    const clients = new Map<string, ObjectClient>();
    for (const name of PermissionScope.options) {
        const opened = view.opens(name);
        const scope = context[name];
        if (opened.length > 0 && scope !== undefined) {
            const client = await openScope(scope, opened, context.user, signal);
            for (const call of objects[name] ?? []) {
                const object = opened.find((type) => type.name === call.object.name);
                if (object === undefined) {
                    throw new TypeError(
                        `view ${view.name} opens no ${call.object.name} in its ${name}`,
                    );
                }
                await client.call(object, call.method, call.input).predicted;
            }
            clients.set(scope, client);
        }
    }

    // mount the declared services offline beside the scopes
    const services = new Map(view.services.map((service) => [service.package.id, OFFLINE]));
    const { default: component } = await view.component();

    return { component, mount: { context, clients, services } };
}

/** Open one scope's objects in an in-memory database whose service never answers, closing both on abort. */
async function openScope(
    scope: string,
    objects: readonly ObjectType[],
    caller: ViewContext["user"],
    signal: AbortSignal,
): Promise<ObjectClient> {
    // keep the objects in memory
    const named = Object.fromEntries(objects.map((object) => [object.name, object]));
    const memory = await WasmClient.memory();
    const database = new SqliteDatabase(memory, ObjectClient.tables(named), "embedded", undefined);

    // reach a service that never answers
    const client = await ObjectClient.open({
        database,
        objects: named,
        scope,
        caller,
        endpoint: OFFLINE,
        reconnect: () => OFFLINE,
    });
    signal.addEventListener("abort", () => {
        void client.close().then(() => memory.close());
    });

    return client;
}
