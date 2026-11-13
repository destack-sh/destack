import { type Attributes, type Context, createContextKey } from "@opentelemetry/api";

/** The attributes that name a measurement's target: names, types, kinds, methods and namespaces. */
const NAMING = /\.(?:name|type|kind|method|namespace)$/u;

/** The attributes naming the system or the outcome rather than the target, such as the engine. */
const EXCLUDED: ReadonlySet<string> = new Set(["db.system.name", "rpc.system.name", "error.type"]);

/** The context key of the innermost span running. */
const PATH_KEY = createContextKey("destack.span_path");

/** The separator of a span path's labels. */
const SEPARATOR = ">";

/** One running span: its path from the outermost, the span it runs in, and what ran inside it. */
class SpanEntry {
    /** The span's path: the labels of the spans it runs in, outermost first, and its own. */
    readonly path: string;
    /** The span it runs in, absent for the outermost. */
    readonly parent: SpanEntry | undefined;
    /** What ran inside the span, nested spans included, by attribute name. */
    readonly totals = new Map<string, number>();

    /** Enter a span below another, or as the outermost. */
    constructor(label: string, parent: SpanEntry | undefined) {
        this.path = parent === undefined ? label : `${parent.path}${SEPARATOR}${label}`;
        this.parent = parent;
    }
}

/** The paths of the spans running code, each span labelled by its name and target. */
export const SpanPath = {
    /** Label a span: its name, then the values of the attributes naming its target. */
    label(name: string, attributes: Attributes): string {
        const named = SpanPath.naming(attributes);

        return named === "" || named === name ? name : `${name}[${named}]`;
    },

    /** Join the values of the attributes that name a measurement's target. */
    naming(attributes: Attributes): string {
        return Object.entries(attributes)
            .filter(([key]) => NAMING.test(key) && !EXCLUDED.has(key))
            .map(([, value]) => String(value))
            .join(":");
    },

    /** Enter a span in a context below the spans already running there, with empty totals. */
    enter(context: Context, label: string): Context {
        return context.setValue(PATH_KEY, new SpanEntry(label, innermost(context)));
    },

    /** Read the span path running in a context, outermost first, absent outside any span. */
    of(context: Context): string | undefined {
        return innermost(context)?.path;
    },

    /** Add to a total, such as statements run, of every span running in a context. */
    add(context: Context, key: string, value: number): void {
        for (let entry = innermost(context); entry !== undefined; entry = entry.parent) {
            entry.totals.set(key, (entry.totals.get(key) ?? 0) + value);
        }
    },

    /** Read the totals of the innermost span running in a context, as span attributes. */
    totals(context: Context): Attributes {
        return Object.fromEntries(innermost(context)?.totals ?? []);
    },
};

/** Read the innermost span running in a context, absent outside any. */
function innermost(context: Context): SpanEntry | undefined {
    const value = context.getValue(PATH_KEY);

    return value instanceof SpanEntry ? value : undefined;
}
