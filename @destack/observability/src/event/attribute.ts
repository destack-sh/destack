import { defineSchema, schema } from "@destack/schema";

/** A primitive attribute value, as OpenTelemetry attributes carry them. */
const Primitive = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** An attribute value: a primitive or a list of primitives. */
export const AttributeValue = defineSchema(schema.union([Primitive, schema.array(Primitive)]));
/** An attribute value. */
export type AttributeValue = schema.Infer<typeof AttributeValue>;

/** The attributes of an event, by key. */
export const Attributes = defineSchema(schema.record(schema.string(), AttributeValue));
/** The attributes of an event. */
export type Attributes = schema.Infer<typeof Attributes>;

/** The attributes an event is queried by, one primitive each, lists written as JSON. */
export const AttributeKey = schema.record(schema.string(), Primitive);
/** The attributes an event is queried by. */
export type AttributeKey = schema.Infer<typeof AttributeKey>;

/** The library or package that recorded an event, as OpenTelemetry names its instrumentation scope. */
export const Instrumentation = defineSchema(
    schema.object({
        /** The library or package name. */
        name: schema.string(),
        /** Its version. */
        version: schema.string(),
    }),
);
/** The library or package that recorded an event. */
export type Instrumentation = schema.Infer<typeof Instrumentation>;

/** The attributes only a person's key opens and only readers holding unmask see, a schema apart from the plain attributes it marks. */
export const PersonalAttributes = schema.sensitive(Attributes, "personal");

/** The prefix of attribute keys an emitter marks personal. */
export const SENSITIVE_PREFIX = "sensitive.";

/** The attribute naming the person a signal acted for, from the OpenTelemetry semantic conventions. */
export const PERSON_ATTRIBUTE = "enduser.id";
