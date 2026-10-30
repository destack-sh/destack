import type { Environment } from "@destack/environment";
import { principal } from "@destack/access";
import { WasmClient } from "@destack/db/browser";
import { SqliteDatabase } from "@destack/db/sqlite";
import type { ObjectType } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import type { ExampleObjects } from "@destack/package/declare";
import { Scope } from "@destack/sync";
import { CONTEXT_SCOPES, type ViewContext } from "../declare/context.ts";
import type { View } from "../declare/view.ts";
import { type Component, type JSX, createComponent } from "../solid/component.ts";
import { Loading } from "../solid/flow.ts";
import { createMemo, onCleanup } from "../solid/reactive.ts";
import { useEnvironment, type ViewMount, ViewMountContext } from "../page/view.ts";
import { ViewError } from "../error/error.ts";

/** The sample space a sample view opens in. */
const SAMPLE_SPACE = "space-01996ab0-0000-7000-8000-00000000e001";

/** The sample person's home. */
const SAMPLE_HOME = "space-01996ab0-0000-7000-8000-00000000e002";

/** The account of the sample space. */
const SAMPLE_ACCOUNT = "account-01996ab0-0000-7000-8000-00000000e003";

/** The sample person. */
const SAMPLE_USER = "user-01996ab0-0000-7000-8000-00000000e004";

/** The address of the sample space's packages, which never answer. */
const OFFLINE = { url: "https://sample.invalid", fetch: () => new Promise<Response>(() => {}) };

/** What a view's example renders. */
interface ViewExampleProperties {
    /** The view to render. */
    readonly view: View;
    /** The calls by scope, each the person's own local change. */
    readonly objects: ExampleObjects;
}

/** Render a view's example offline over the objects its calls bring about, each scope in memory. */
export function ViewExample(properties: ViewExampleProperties): JSX.Element {
    // open the scopes and load the view's component as the world's work, closing the scopes on unmount
    const environment = useEnvironment();
    const closing = new AbortController();
    onCleanup(() => closing.abort());
    const opened = createMemo(() =>
        environment.scheduler.track(openSample(properties, environment, closing.signal), "world"),
    );

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
    environment: Environment,
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
        user: principal.user.reference(Scope.universe.id, SAMPLE_USER),
    };

    // open each scope the context opens, and make the example's calls there
    const clients = new Map<string, ObjectClient>();
    for (const name of CONTEXT_SCOPES) {
        const opened = view.opens(name);
        const scope = context[name];
        if (opened.length > 0 && scope !== undefined) {
            const client = await openScope(scope, opened, context.user, environment, signal);
            for (const call of objects[name] ?? []) {
                const object = opened.find((type) => type.name === call.object.name);
                if (object === undefined) {
                    throw ViewError.of(
                        "OBJECT_NOT_OPENED",
                        `view ${view.name} does not open ${call.object.name} in its ${name}`,
                    );
                }
                await client.call(object, call.method, call.input).predicted;
            }
            clients.set(scope, client);
        }
    }

    // mount the declared packages offline beside the scopes
    const packages = new Map(view.packages.map((handle) => [handle.id, OFFLINE]));
    const { default: component } = await view.component();

    return {
        component,
        mount: {
            context,
            clients,
            packages,
            origin: () => OFFLINE,
        },
    };
}

/** Open one scope's objects in an in-memory database whose service never answers. */
async function openScope(
    scope: string,
    objects: readonly ObjectType[],
    caller: ViewContext["user"],
    environment: Environment,
    signal: AbortSignal,
): Promise<ObjectClient> {
    // keep the objects in memory
    const named = Object.fromEntries(objects.map((object) => [object.name, object]));
    const memory = await WasmClient.memory(environment);
    const database = new SqliteDatabase(memory, ObjectClient.tables(named), "embedded", {
        environment,
    });

    // reach a service that never answers
    const client = await ObjectClient.open({
        database,
        objects: named,
        scope,
        caller,
        endpoint: OFFLINE,
        reconnect: () => OFFLINE,
        environment,
    });
    signal.addEventListener("abort", () => {
        void client.close().then(() => memory.close());
    });

    return client;
}
