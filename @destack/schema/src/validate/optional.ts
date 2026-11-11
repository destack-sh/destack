import { ZodType } from "zod";

/** Decide whether an object may leave a schema's key out, as the schema's objects decide it. */
export function isOptional(type: ZodType): boolean {
    return type._zod.optin !== undefined;
}

/** Read the schema an optional, nullable or defaulted schema wraps, the schema itself otherwise. */
export function unwrap(type: ZodType): ZodType {
    const inner: unknown = Reflect.get(type._zod.def, "innerType");

    return inner instanceof ZodType ? unwrap(inner) : type;
}
