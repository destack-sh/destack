import { schema } from "@destack/schema";

/** The module filter Start applies when it collects development CSS. */
const FilterPattern = schema.union([
    schema.string(),
    schema.instanceof(RegExp),
    schema.array(schema.union([schema.string(), schema.instanceof(RegExp)])).readonly(),
    schema.null(),
]);

/** A Solid application: its root component and the framework modules around it. */
export const SolidApplication = schema.object({
    /** The package-relative module whose default export is the application's root component. */
    app: schema.string(),
    /** The document shell component wrapping the app in generated entries. */
    document: schema.string().exactOptional(),
    /** The client entry module. */
    entryClient: schema.string().exactOptional(),
    /** The server entry module. */
    entryServer: schema.string().exactOptional(),
    /** The request middleware module. */
    middleware: schema.string().exactOptional(),
    /** The module run once before the server handles requests. */
    setup: schema.string().exactOptional(),
    /** The render mode Start knows, or the path of the module choosing one. */
    renderMode: schema.string().exactOptional(),
    /** The development CSS crawling options. */
    css: schema
        .object({
            /** The modules traversed while collecting development CSS. */
            filter: schema
                .object({
                    /** The modules opted back in. */
                    include: FilterPattern.exactOptional(),
                    /** The modules pruned. */
                    exclude: FilterPattern.exactOptional(),
                })
                .exactOptional(),
        })
        .exactOptional(),
});
/** A Solid application: its root component and the framework modules around it. */
export type SolidApplication = schema.Infer<typeof SolidApplication>;
