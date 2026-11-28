import { type Attributes, type Context, createContextKey } from "@opentelemetry/api";

/** The attributes that name a measurement's target: names, types, kinds, methods and namespaces. */
const TARGET = /\.(?:name|type|kind|method|namespace)$/u;

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
    /** Whether the span ended, taking no more totals. */
    isEnded = false;

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
        const target = SpanPath.target(attributes);

        return target === "" || target === name ? name : `${name}[${target}]`;
    },

    /** Read a measurement's target: the values of the attributes that name it, joined. */
    target(attributes: Attributes): string {
        // join the values in one pass, as every span labels itself
        let target: string | undefined = undefined;
        for (const key in attributes) {
            if (TARGET.test(key) && !EXCLUDED.has(key)) {
                const value = String(attributes[key]);
                target = target === undefined ? value : `${target}:${value}`;
            }
        }

        return target ?? "";
    },

    /** Enter a span in a context below the spans already running there, with empty totals. */
    enter(context: Context, label: string): Context {
        return context.setValue(PATH_KEY, new SpanEntry(label, innermost(context)));
    },

    /** Decide whether a context runs inside the innermost span of another, or in that span. */
    isWithin(context: Context, outer: Context): boolean {
        // walk up from the inner context's span, looking for the outer one's
        const target = innermost(outer);
        for (let entry = innermost(context); entry !== undefined; entry = entry.parent) {
            if (entry === target) {
                return true;
            }
        }

        return false;
    },

    /** Read the span path running in a context, outermost first, absent outside any span. */
    of(context: Context): string | undefined {
        return innermost(context)?.path;
    },

    /** Add to totals, such as statements run, of every span running in a context, in one walk. */
    add(context: Context, totals: Readonly<Record<string, number>>): void {
        const added = Object.entries(totals);
        for (let entry = innermost(context); entry !== undefined; entry = entry.parent) {
            for (const [key, value] of added) {
                entry.totals.set(key, (entry.totals.get(key) ?? 0) + value);
            }
        }
    },

    /** Read the totals of the innermost span running in a context, as span attributes. */
    totals(context: Context): Attributes {
        return Object.fromEntries(innermost(context)?.totals ?? []);
    },

    /** End the span a context entered, so work outliving it counts to the spans still running. */
    end(context: Context): void {
        const entry = context.getValue(PATH_KEY);
        if (entry instanceof SpanEntry) {
            entry.isEnded = true;
        }
    },
};

/** Read the innermost span still running in a context, absent outside any. */
function innermost(context: Context): SpanEntry | undefined {
    // skip the spans that ended while work they started runs on
    const value = context.getValue(PATH_KEY);
    let entry = value instanceof SpanEntry ? value : undefined;
    while (entry?.isEnded === true) {
        entry = entry.parent;
    }

    return entry;
}
