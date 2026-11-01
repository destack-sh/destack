import { WorldStep } from "@destack/environment";
import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A computer of a world scenario by its name, such as `laptop`. */
const ComputerName = schema.string().min(1);

/** An object type of a world's packages by name, such as `issue`. */
const ObjectName = schema.string().min(1);

/** A step of machine's world driver: computers and members joining, and calls on objects. */
export const MachineStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Start a personal computer, sign the owner in and enroll it in the owner's account. */
        action: schema.literal("open"),
        /** The computer's name. */
        computer: ComputerName,
    }),
    schema.object({
        /** Build a fixture package of the machine's tests and install it in the owner's home. */
        action: schema.literal("install"),
        /** The fixture's directory name below `tests/fixture`, such as `apps`. */
        fixture: schema.string().regex(/^[a-z][a-z0-9-]*$/u),
    }),
    schema.object({
        /** Sign members in on the home computer, each following the objects on synced devices. */
        action: schema.literal("join"),
        /** The members. */
        members: schema.number().int().positive(),
        /** The devices of each member, each an object client with its own replica. */
        devices: schema.number().int().positive(),
        /** The member by index who opens more devices, new members when absent. */
        member: schema.number().int().nonnegative().exactOptional(),
    }),
    schema.object({
        /** Grant every member a relation on the object of a type created last. */
        action: schema.literal("share"),
        /** The object type. */
        object: ObjectName,
        /** The relation, such as `member`. */
        relation: schema.string().min(1),
    }),
    schema.object({
        /** Revoke a member's relation on a type's latest object, until their devices lose it. */
        action: schema.literal("unshare"),
        /** The object type. */
        object: ObjectName,
        /** The member by index. */
        member: schema.number().int().nonnegative(),
    }),
    schema.object({
        /** Call an object method, then wait until every device's replica has the result. */
        action: schema.literal("call"),
        /** The object type. */
        object: ObjectName,
        /** The method, such as `create`. */
        method: schema.string().min(1),
        /** The type whose object created last is the parent of a created object. */
        in: ObjectName.exactOptional(),
        /** The type whose object created last the method targets. */
        on: ObjectName.exactOptional(),
        /** The member calling by index, the owner when absent. */
        member: schema.number().int().nonnegative().exactOptional(),
        /** The input, `{n}` replaced by the call's number: a number for a whole string. */
        input: schema.jsonObject(),
        /** The calls, made one after another at the rate and waited for once, one when absent. */
        repeat: schema.number().int().positive().exactOptional(),
        /** The calls a second of world time, back to back when absent. */
        rate: schema.number().positive().exactOptional(),
    }),
    schema.object({
        /** Send unstored events on a type's latest object until every reader's device has them. */
        action: schema.literal("broadcast"),
        /** The object type. */
        object: ObjectName,
        /** The member sending from their first device, by index. */
        member: schema.number().int().nonnegative(),
        /** The event, `{n}` replaced by the event's number: a number for a whole string. */
        event: schema.jsonObject(),
        /** The events, sent one after another at the rate. */
        repeat: schema.number().int().positive(),
        /** The events a second of world time. */
        rate: schema.number().positive(),
    }),
    schema.object({
        /** Type into a text field of a type's latest object, members at once, an edit a key. */
        action: schema.literal("type"),
        /** The object type. */
        object: ObjectName,
        /** The text field. */
        field: schema.string().min(1),
        /** The text each member types at the field's end. */
        text: schema.string().min(1),
        /** The members typing at once, from their first devices, the first ones by index. */
        members: schema.number().int().positive(),
    }),
]);
/** A step of machine's world driver. */
export type MachineStep = schema.Infer<typeof MachineStep>;

/** A step of the world interaction: one every world shares, or machine's own. */
export const WorldInteractionStep = schema.union([WorldStep, MachineStep]);
/** A step of the world interaction. */
export type WorldInteractionStep = WorldStep | MachineStep;

/** An observation of a world: the replicas a computer follows. */
export const WorldObservation = schema.discriminatedUnion("kind", [
    schema.object({
        /** The replicas a computer's machine and device databases follow. */
        kind: schema.literal("replicas"),
        /** The computer's name, `home` for the owner's home. */
        computer: ComputerName,
    }),
]);
/** An observation of a world. */
export type WorldObservation = schema.Infer<typeof WorldObservation>;

/** The world interaction: computers, people and time in a simulated universe. */
export const worldInteraction: Interaction<
    WorldInteractionStep,
    WorldObservation,
    Record<string, never>
> = Object.freeze({
    name: "world",
    step: WorldInteractionStep,
    observation: WorldObservation,
    globals: schema.object({}),
});
