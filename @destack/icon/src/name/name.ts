import { schema } from "@destack/schema";
import { ICON_NAMES } from "../phosphor/names.ts";

/** The name of a Phosphor icon, such as `book-open`. */
export const IconName = schema.enum(ICON_NAMES);
/** The name of a Phosphor icon. */
export type IconName = schema.Infer<typeof IconName>;
