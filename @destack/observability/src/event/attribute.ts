import { schema } from "@destack/schema";

/** A primitive attribute value, as OpenTelemetry attributes carry them. */
const Primitive = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** An attribute value: a primitive or a list of primitives. */
export const AttributeValue = schema.union([Primitive, schema.array(Primitive)]);
/** An attribute value. */
export type AttributeValue = schema.Infer<typeof AttributeValue>;

/** The attributes of an event, by key. */
export const Attributes = schema.record(schema.string(), AttributeValue);
/** The attributes of an event. */
export type Attributes = schema.Infer<typeof Attributes>;

/** The attributes an event is queried by, one primitive each, lists written as JSON. */
export const AttributeKey = schema.record(schema.string(), Primitive);
/** The attributes an event is queried by. */
export type AttributeKey = schema.Infer<typeof AttributeKey>;

/** The library or package that recorded an event. */
export const Instrumentation = schema.object({
    /** The library or package name. */
    name: schema.string(),
    /** Its version. */
    version: schema.string(),
});
/** The library or package that recorded an event. */
export type Instrumentation = schema.Infer<typeof Instrumentation>;

/** The attributes only a person's key opens and only readers with unmask see. */
export const PersonalAttributes = schema.sensitive(Attributes, "personal");

/** The prefix of attribute keys an emitter marks personal. */
export const SENSITIVE_PREFIX = "sensitive.";

/** The attribute of the person a signal acted for. */
export const PERSON_ATTRIBUTE = "enduser.id";

/** Read an attribute value as text, absent unless it is a non-empty string. */
export function textOf(value: Attributes[string] | undefined): string | undefined {
    return typeof value === "string" && value !== "" ? value : undefined;
}
