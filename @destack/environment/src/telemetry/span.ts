import { type Attributes, type Context, createContextKey } from "@opentelemetry/api";

/** The attributes that name a measurement's target: names, kinds, methods and namespaces. */
const NAMING = /\.(?:name|kind|method|namespace)$/u;

/** The attributes naming the system rather than the target, such as the database's engine. */
const SYSTEMS: ReadonlySet<string> = new Set(["db.system.name", "rpc.system.name"]);

/** The context key of the span path, outermost first. */
const PATH_KEY = createContextKey("destack.span_path");

/** The separator of a span path's labels. */
const SEPARATOR = ">";

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
            .filter(([key]) => NAMING.test(key) && !SYSTEMS.has(key))
            .map(([, value]) => String(value))
            .join(":");
    },

    /** Enter a span's label in a context below the spans already running there. */
    enter(context: Context, label: string): Context {
        const outer = SpanPath.of(context);

        return context.setValue(
            PATH_KEY,
            outer === undefined ? label : `${outer}${SEPARATOR}${label}`,
        );
    },

    /** Read the span path running in a context, outermost first, absent outside any span. */
    of(context: Context): string | undefined {
        const label = context.getValue(PATH_KEY);

        return typeof label === "string" ? label : undefined;
    },
};
