import { LogPosition } from "@destack/db/log";
import { identifier, schema } from "@destack/schema";
import { defineProcedure, eventIterator } from "@destack/service";
import { QueryPage, ObjectTypeReference } from "@destack/sync";

/** The most users one profile stream follows, a large workspace. */
const PROFILE_LIMIT = 10_000;

/** A user's public profile. */
export const Profile = schema.object({
    /** The user. */
    id: identifier("user"),
    /** The display name. */
    name: schema.string(),
    /** The profile image URL, absent without one. */
    image: schema.string().nullable(),
});
/** A user's public profile. */
export type Profile = schema.Infer<typeof Profile>;

/** The handle a user goes by. */
export const Handle = schema.object({
    /** The personal account. */
    accountId: identifier("account"),
    /** The user owning it. */
    userId: identifier("user"),
    /** The account's handle. */
    handle: schema.string(),
});
/** The handle a user goes by. */
export type Handle = schema.Infer<typeof Handle>;

/** A page of profile and handle changes. */
export const ProfilePage = schema.object({
    /** Whether the page starts a snapshot, replacing everything the copy holds. */
    reset: schema.boolean(),
    /** Whether the copy holds every requested profile after it applies the page. */
    complete: schema.boolean(),
    /** The log position to continue after once the page completes. */
    position: LogPosition,
    /** The profiles entering or changing. */
    profiles: schema.array(Profile),
    /** The users whose profiles leave. */
    removed: schema.array(identifier("user")),
    /** The handles entering or changing. */
    handles: schema.array(Handle),
    /** The personal accounts whose handles leave. */
    unhandled: schema.array(identifier("account")),
});
/** A page of public profiles and handles. */
export type ProfilePage = schema.Infer<typeof ProfilePage>;

/** The access and profiles the cell serving a space copies. */
export const replica = {
    /** Follow the access of one scope above a served space or above the account a host serves. */
    watch: defineProcedure({
        authentication: "identity",
        permission: null,
        audit: false,
    })
        .route({ method: "POST", path: "/access" })
        .input(
            schema.object({
                /** The space the follower serves, absent for the account the calling host serves. */
                spaceId: identifier("space").optional(),
                /** The scope above the space or account. */
                scope: schema.string().min(1),
                /** The object types the follower holds the access rows of itself. */
                held: schema.array(ObjectTypeReference),
                /** The object types whose inherited rows the follower copies. */
                copied: schema.array(ObjectTypeReference),
                /** The log position the follower holds, absent before its first snapshot. */
                after: LogPosition.optional(),
            }),
        )
        .output(eventIterator(QueryPage)),
    /** Follow the public profiles and handles of a space's members. */
    profiles: defineProcedure({
        authentication: "identity",
        permission: null,
        audit: false,
    })
        .route({ method: "POST", path: "/spaces/{spaceId}/profiles" })
        .input(
            schema.object({
                /** The space whose cell follows. */
                spaceId: identifier("space"),
                /** The users the space's access refers to. */
                users: schema.array(identifier("user")).max(PROFILE_LIMIT),
                /** The users the copy followed up to its position, when they differ. */
                previous: schema.array(identifier("user")).max(PROFILE_LIMIT).optional(),
                /** The log position the copy holds, absent before its first snapshot. */
                after: LogPosition.optional(),
            }),
        )
        .output(eventIterator(ProfilePage)),
};
