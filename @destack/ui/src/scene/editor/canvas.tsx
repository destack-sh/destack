import { t } from "@destack/locale";
import { type ActionInput, Choice } from "@destack/object";
import { OriginPath } from "@destack/package";
import type { JsonObject } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    createMemo,
    For,
    type JSX,
    onCleanup,
    ExampleRequest,
    onSettled,
    ExampleEvent,
    type MatrixAxes,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "../../select/index.ts";

/** The widths of the viewports the canvas frames a page at, in CSS pixels, none to fill it. */
export const VIEWPORTS = { fill: undefined, phone: 390, tablet: 768, desktop: 1280 } as const;

/** A viewport the canvas frames a page at. */
export type Viewport = keyof typeof VIEWPORTS;

/** The styles of the canvas and its frames. */
const styles = style.create({
    canvas: {
        height: "100%",
        overflow: "auto",
        backgroundColor: color.muted,
    },
    stage: {
        display: "grid",
        justifyItems: "center",
        minHeight: "100%",
        padding: space[6],
        boxSizing: "border-box",
    },
    page: {
        display: "block",
        width: "100%",
        height: "100%",
        minHeight: "32rem",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.background,
        boxShadow: shadow.raised,
        transformOrigin: "top center",
    },
    matrix: {
        display: "block",
        width: "100%",
        height: "100%",
        borderWidth: 0,
        backgroundColor: color.muted,
    },
    axis: {
        display: "inline-flex",
        alignItems: "center",
        color: color.mutedForeground,
    },
});

/** Address the page of an example with its globals, starting properties and matrix axes. */
export function pageOf(
    name: string,
    globals: JsonObject,
    properties: JsonObject = {},
    matrix?: MatrixAxes,
): string {
    // write the name and globals, and the properties and axes where there are any
    const query = new URLSearchParams({ name, globals: JSON.stringify(globals) });
    if (Object.keys(properties).length > 0) {
        query.set("properties", JSON.stringify(properties));
    }
    if (matrix !== undefined) {
        query.set("matrix", JSON.stringify(matrix));
    }

    return `${OriginPath.example}?${query}`;
}

/** Frame an example's page on the canvas at a viewport's width and a zoom. */
export function Canvas(properties: {
    /** The page's address. */
    readonly source: string;
    /** The page's title. */
    readonly title: string;
    /** The viewport. */
    readonly viewport: Viewport;
    /** The zoom, 1 at the page's own size. */
    readonly zoom: number;
    /** Take the page's frame, to talk to its page. */
    readonly onFrame: (frame: HTMLIFrameElement) => void;
}): JSX.Element {
    return (
        <div {...style.attrs(styles.canvas)}>
            <div {...style.attrs(styles.stage)}>
                <iframe
                    ref={properties.onFrame}
                    title={properties.title}
                    src={properties.source}
                    {...style.attributes(styles.page, {
                        "max-width":
                            VIEWPORTS[properties.viewport] === undefined
                                ? "100%"
                                : `${VIEWPORTS[properties.viewport]}px`,
                        transform: `scale(${properties.zoom})`,
                    })}
                />
            </div>
        </div>
    );
}

/** List the inputs a matrix lays an example out along: its choices and its switches. */
export function axesOf(inputs: readonly ActionInput[]): readonly ActionInput[] {
    return inputs.filter(
        (input) => input.value.kind === "choice" || input.value.kind === "boolean",
    );
}

/** Lay an example out along one or two of its choices, as columns and rows, in one page. */
export function Matrix(properties: {
    /** The example's name. */
    readonly name: string;
    /** The globals its page renders with. */
    readonly globals: JsonObject;
    /** The value of each input. */
    readonly values: JsonObject;
    /** The input the columns vary. */
    readonly columns: ActionInput;
    /** The input the rows vary, none for one row. */
    readonly rows: ActionInput | undefined;
    /** Handle a person choosing a cell: the values it lays out. */
    readonly onChoose: (values: JsonObject) => void;
    /** Handle a person opening a cell on its own: the values it lays out. */
    readonly onOpen: (values: JsonObject) => void;
}): JSX.Element {
    // title each value in the person's language
    const localization = useLocale();
    const valuesOf = (input: ActionInput) =>
        input.value.kind === "choice"
            ? input.value.choices.map((each) => {
                  const { value, title } = Choice.of(each, localization);

                  return { value, title };
              })
            : [
                  { value: false, title: localization.render(t.context("option")`Off`) },
                  { value: true, title: localization.render(t.context("option")`On`) },
              ];

    // lay the matrix out along the columns' values, and the rows' where it has rows
    const axes = createMemo(() => {
        const row = properties.rows;

        return {
            columns: { name: properties.columns.name, values: valuesOf(properties.columns) },
            ...(row === undefined ? {} : { rows: { name: row.name, values: valuesOf(row) } }),
        };
    });

    // talk to the page at the editor's own origin
    const { origin } = new URL(useEnvironment().navigation.currentEntry.url);
    const tell = (request: ExampleRequest) => {
        const target = page?.contentWindow;
        if (target !== null && target !== undefined) {
            ExampleRequest.send(target, request, origin);
        }
    };

    // load the page once, then show other examples and globals, values and axes in it in place
    const source = untrack(() =>
        pageOf(properties.name, properties.globals, properties.values, axes()),
    );
    let page: HTMLIFrameElement | undefined;
    createEffect(
        () => ({ name: properties.name, globals: properties.globals }),
        ({ name, globals }, previous) => {
            if (previous !== undefined) {
                const address = {
                    name,
                    globals,
                    properties: untrack(() => properties.values),
                    matrix: untrack(axes),
                };
                tell({ destack: "show", ...address });
            }
        },
    );
    const send = () => {
        tell({ destack: "axes", axes: axes() });
        tell({ destack: "set", properties: properties.values });
    };
    createEffect(
        () => [axes(), properties.values],
        () => send(),
    );

    // take the cell its page says a person picked or opened
    const listen = (event: MessageEvent<unknown>) => {
        const received =
            event.source === page?.contentWindow ? ExampleEvent.read(event, origin) : undefined;
        // open the cell on its own
        if (received?.destack === "open") {
            properties.onOpen(received.values);
        }
        // pick the cell
        else if (received?.destack === "cell") {
            properties.onChoose(received.values);
        }
    };
    onSettled(() => {
        window.addEventListener("message", listen);
        onCleanup(() => window.removeEventListener("message", listen));
    });

    return (
        <iframe
            ref={page}
            title={localization.render(t.context("title")`Matrix`)}
            src={source}
            onLoad={send}
            {...style.attrs(styles.matrix)}
        />
    );
}

/** Render a toolbar's quiet choice of the input an axis of the matrix lays out along. */
export function AxisChoice(properties: {
    /** The axis, such as Columns. */
    readonly label: string;
    /** The axis's icon. */
    readonly icon: JSX.Element;
    /** The input laid out, or none. */
    readonly value: string;
    /** The inputs to lay out along, by name and label, none first where the axis may go unused. */
    readonly axes: readonly { readonly name: string; readonly label: string }[];
    /** Handle another input being chosen. */
    readonly onChange: (name: string) => void;
}): JSX.Element {
    return (
        <span {...style.attrs(styles.axis)}>
            {properties.icon}
            <Select
                size="sm"
                variant="ghost"
                aria-label={properties.label}
                title={properties.label}
                value={properties.value}
                onValueChange={(value) => {
                    if (value !== undefined) {
                        properties.onChange(value);
                    }
                }}
            >
                <SelectTrigger>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <For each={properties.axes}>
                        {(axis) => <SelectItem value={axis.name}>{axis.label}</SelectItem>}
                    </For>
                </SelectContent>
            </Select>
        </span>
    );
}
