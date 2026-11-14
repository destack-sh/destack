import { WorldStep } from "@destack/environment";
import type { Interaction } from "@destack/package/declare";
import { ObjectStep } from "@destack/object/scenario";
import { schema } from "@destack/schema";

/** A computer of a world scenario by its name, such as `laptop`. */
const ComputerName = schema.string().min(1);

/** An object type of a world's packages by name, such as `issue`. */
const ObjectName = schema.string().min(1);

/** A fixture package of the machine's tests by its directory name, such as `apps`. */
const FixtureName = schema.string().regex(/^[a-z][a-z0-9-]*$/u);

/** A person of a world by name: `owner`, or a member such as `member-0`. */
const Person = schema.string().regex(/^(owner|member-\d+)$/u);

/** A machine action: computers, people, releases and spaces changing in the world. */
export const MachineAction = schema.discriminatedUnion("action", [
    schema.object({
        /** Start a personal computer, sign the owner in and enroll it in the owner's account. */
        action: schema.literal("open"),
        /** The computer's name. */
        computer: ComputerName,
    }),
    schema.object({
        /** Sign a person in on the home computer, signing them up on their first sign-in. */
        action: schema.literal("sign-in"),
        /** The person's email, `{n}` replaced by the step's number. */
        email: schema.string().min(1),
    }),
    schema.object({
        /** Publish a fixture package's build as the platform's operator. */
        action: schema.literal("publish"),
        /** The fixture. */
        fixture: FixtureName,
    }),
    schema.object({
        /** Install a published fixture package in the owner's home, until the home applies it. */
        action: schema.literal("install"),
        /** The fixture. */
        fixture: FixtureName,
    }),
    schema.object({
        /** Update an installation in the owner's home to a published release, until it applies. */
        action: schema.literal("update"),
        /** The installation's name, such as `apps`. */
        installation: schema.string().min(1),
        /** The release, by the run's lock of a fixture such as `apps-next`. */
        release: FixtureName,
    }),
    schema.object({
        /** Sign members in on the home computer, each following the objects on synced devices. */
        action: schema.literal("join"),
        /** The members. */
        members: schema.number().int().positive(),
        /** The devices of each member, each a client with its own replica, none to only sign in. */
        devices: schema.number().int().nonnegative(),
    }),
    schema.object({
        /** Connect more devices of a joined member, each following the objects. */
        action: schema.literal("connect"),
        /** The member, such as `member-0`. */
        member: Person,
        /** The devices to connect. */
        devices: schema.number().int().positive(),
    }),
    schema.object({
        /** Launch an installation's view in the owner's home as the owner until it loads. */
        action: schema.literal("launch"),
        /** The installation's name, such as `start`. */
        installation: schema.string().min(1),
        /** The view's name, such as `start`. */
        view: schema.string().min(1),
    }),
    schema.object({
        /** Move a created space to a computer as its stepped-up owner, until it serves there. */
        action: schema.literal("move"),
        /** The space's name, as it was created. */
        space: schema.string().min(1),
        /** The computer's name. */
        computer: ComputerName,
    }),
    schema.object({
        /** Restart the home's computer, keeping its state, until it serves the home again. */
        action: schema.literal("restart"),
    }),
    schema.object({
        /** Invite a member to own a created space, as the owner. */
        action: schema.literal("invite"),
        /** The member, such as `member-0`. */
        member: Person,
        /** The space's name, as it was created. */
        space: schema.string().min(1),
    }),
    schema.object({
        /** Accept a member's latest invitation as them. */
        action: schema.literal("accept"),
        /** The member, such as `member-0`. */
        member: Person,
    }),
    schema.object({
        /** Create a space in the owner's account, waiting until the home computer serves it. */
        action: schema.literal("create"),
        /** The space's name, such as `team-{n}`. */
        space: schema.string().min(1),
    }),
    schema.object({
        /** Send unstored events on an object from a member's device until its holders have them. */
        action: schema.literal("broadcast"),
        /** The object type. */
        object: ObjectName,
        /** The object, as a call bound it, such as `$shape`. */
        on: schema.string().min(1),
        /** The member sending, such as `member-0`. */
        as: Person,
        /** The event. */
        event: schema.jsonObject(),
        /** The events, sent one after another at the rate. */
        repeat: schema.number().int().positive(),
        /** The events a second of world time. */
        rate: schema.number().positive(),
    }),
    schema.object({
        /** Type into an object's text field, members at once, until its holders show it. */
        action: schema.literal("type"),
        /** The object type. */
        object: ObjectName,
        /** The object, as a call bound it, such as `$page`. */
        on: schema.string().min(1),
        /** The text field. */
        field: schema.string().min(1),
        /** The text each member types at the field's end. */
        text: schema.string().min(1),
        /** The members typing at once, from their first devices, the first ones by index. */
        members: schema.number().int().positive(),
    }),
]);
/** A machine action. */
export type MachineAction = schema.Infer<typeof MachineAction>;

/** A call of an object's method in the owner's home, a burst of calls at a rate when repeated. */
export const MachineCall = ObjectStep.extend({
    /** The person calling: `owner`, or a member such as `member-0`. */
    as: Person,
    /** The calls, made one after another at the rate and settled once, one when absent. */
    repeat: schema.number().int().positive().exactOptional(),
    /** The calls a second of world time, back to back when absent. */
    rate: schema.number().positive().exactOptional(),
});
/** A call of an object's method in the owner's home. */
export type MachineCall = schema.Infer<typeof MachineCall>;

/** A step of the machine interaction: one every world shares, a call, or a machine action. */
export const MachineStep = schema.union([WorldStep, MachineCall, MachineAction]);
/** A step of the machine interaction. */
export type MachineStep = WorldStep | MachineCall | MachineAction;

/** An observation of a world: the objects of a type a host holds. */
export const MachineObservation = schema.object({
    /** Count the objects of a type a host holds. */
    count: ObjectName,
    /** The host by name, such as `member-0.device-1`, the owner's home when absent. */
    on: schema.string().min(1).exactOptional(),
});
/** An observation of a world. */
export type MachineObservation = schema.Infer<typeof MachineObservation>;

/** The machine interaction: computers, people and time in a simulated universe. */
export const machineInteraction: Interaction<
    MachineStep,
    MachineObservation,
    Record<string, never>
> = Object.freeze({
    name: "machine",
    step: MachineStep,
    observation: MachineObservation,
    globals: schema.object({}),
});
