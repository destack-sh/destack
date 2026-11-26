import { Message } from "@destack/package";
import { SetStep } from "@destack/package/declare";
import { Step } from "../scenario/step.ts";
import { ActionInputDescription } from "@destack/object";
import { schema } from "@destack/schema";

/** A value an axis takes, with its title in the person's language. */
const MatrixValue = schema.object({
    /** The value. */
    value: schema.json(),
    /** The value as people read it. */
    title: schema.string(),
});

/** One axis a matrix lays an example out along: a property and the values it takes. */
const MatrixAxis = schema.object({
    /** The property. */
    name: schema.string().min(1),
    /** Its values, in order. */
    values: schema.array(MatrixValue).min(1),
});

/** The axes a matrix lays an example out along: its columns, and its rows where it has them. */
export const MatrixAxes = schema.object({
    /** The property the columns vary. */
    columns: MatrixAxis,
    /** The property the rows vary, none for one row. */
    rows: MatrixAxis.exactOptional(),
});
/** The axes a matrix lays an example out along. */
export type MatrixAxes = schema.Infer<typeof MatrixAxes>;

/** What a scenario's check saw, and whether that is what it expects. */
export const CheckResult = schema.object({
    /** The value the check saw. */
    seen: schema.json(),
    /** Whether it is the value the check expects. */
    isPassed: schema.boolean(),
});
/** What a scenario's check saw, and whether that is what it expects. */
export type CheckResult = schema.Infer<typeof CheckResult>;

/** A request of a host, such as the editor, to its example page: what to show and how. */
const exampleRequest = schema.discriminatedUnion("destack", [
    schema.object({
        /** Show another example or scenario in place. */
        destack: schema.literal("show"),
        /** Its name, as `<shown>.<member>`. */
        name: schema.string().min(1),
        /** The globals it renders with, as `Globals` reads them. */
        globals: schema.json(),
        /** The properties an example starts from. */
        properties: schema.jsonObject(),
        /** The axes an example's matrix lays it out along, null for the example alone. */
        matrix: MatrixAxes.nullable(),
    }),
    schema.object({
        /** Change the shown example's properties. */
        destack: schema.literal("set"),
        /** The properties and their new values. */
        properties: schema.jsonObject(),
    }),
    schema.object({
        /** Lay the shown matrix out along other axes. */
        destack: schema.literal("axes"),
        /** The axes. */
        axes: MatrixAxes,
    }),
    schema.object({
        /** Act one step of the shown scenario. */
        destack: schema.literal("act"),
        /** The step's position among the scenario's steps. */
        index: schema.number().int().min(0),
    }),
]);

/** An event of an example page for its host: what it shows, what a step saw, what was picked. */
const exampleEvent = schema.discriminatedUnion("destack", [
    schema.object({
        /** An example is shown. */
        destack: schema.literal("example"),
        /** Its name. */
        name: schema.string().min(1),
        /** Its description. */
        description: Message,
        /** The inputs its component's properties take, objects aside. */
        inputs: schema.array(ActionInputDescription),
        /** The value of each input. */
        values: schema.jsonObject(),
    }),
    schema.object({
        /** A scenario is shown. */
        destack: schema.literal("scenario"),
        /** Its name. */
        name: schema.string().min(1),
        /** Its description. */
        description: Message,
        /** Its steps, in order. */
        steps: schema.array(schema.union([SetStep, Step])),
    }),
    schema.object({
        /** A scenario's step ran. */
        destack: schema.literal("step"),
        /** The step's position. */
        index: schema.number().int().min(0),
        /** The result of each of its checks. */
        results: schema.array(CheckResult),
    }),
    schema.object({
        /** A person picked a cell of the matrix, or opened it on its own. */
        destack: schema.enum(["cell", "open"]),
        /** The values the cell lays out. */
        values: schema.jsonObject(),
    }),
]);

/** A request of a host to its example page. */
export const ExampleRequest = Object.assign(exampleRequest, {
    /** Send a request to an example page of an origin. */
    send(page: Window, message: ExampleRequest, origin: string): void {
        page.postMessage(exampleRequest.parse(message), origin);
    },

    /** Read a host's request from an origin, absent for anyone else's, refusing a bad one. */
    read(event: MessageEvent<unknown>, origin: string): ExampleRequest | undefined {
        return isDestack(event, origin) ? exampleRequest.parse(event.data) : undefined;
    },
});
/** A request of a host to its example page. */
export type ExampleRequest = schema.Infer<typeof exampleRequest>;

/** An event of an example page for its host. */
export const ExampleEvent = Object.assign(exampleEvent, {
    /** Send an event to the page's host of an origin, none without a host. */
    send(message: ExampleEvent, origin: string): void {
        if (window.parent !== window) {
            window.parent.postMessage(exampleEvent.parse(message), origin);
        }
    },

    /** Read a page's event from an origin, absent for anyone else's, refusing a bad one. */
    read(event: MessageEvent<unknown>, origin: string): ExampleEvent | undefined {
        return isDestack(event, origin) ? exampleEvent.parse(event.data) : undefined;
    },
});
/** An event of an example page for its host. */
export type ExampleEvent = schema.Infer<typeof exampleEvent>;

/** Report whether an event carries a Destack page's message from an origin. */
function isDestack(event: MessageEvent<unknown>, origin: string): boolean {
    const { data } = event;

    return (
        event.origin === origin && typeof data === "object" && data !== null && "destack" in data
    );
}
