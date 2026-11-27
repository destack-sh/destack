import * as schema from "../validate/index.ts";

/** A web address, as the URL standard parses it. */
export const Url = schema.url().meta({ id: "url", title: "Address" });
/** A web address. */
export type Url = schema.Infer<typeof Url>;
