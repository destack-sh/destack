import { schema } from "@destack/schema";
import { Label } from "./label.ts";

/** The longest handle, sized with `SPACE_NAME_LENGTH` so an address fits one DNS label. */
export const HANDLE_LENGTH = 24;

/** An account's handle: a label of at most `HANDLE_LENGTH` characters. */
export const AccountHandle = Label.schema(HANDLE_LENGTH);
/** An account's handle. */
export type AccountHandle = schema.Infer<typeof AccountHandle>;

/** Whether a handle may be taken. */
export const HandleAvailability = schema.enum(["available", "taken", "invalid"]);
/** Whether a handle may be taken. */
export type HandleAvailability = schema.Infer<typeof HandleAvailability>;
