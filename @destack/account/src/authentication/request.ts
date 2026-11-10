import { GrantPermission, PermissionScope } from "@destack/access";
import { Message } from "@destack/locale";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { HandleAvailability } from "../address/handle.ts";
import { OAUTH_SCOPES } from "../object/application.ts";

/** The OAuth providers people sign in with. */
export const ProviderId = schema.enum(["github", "google", "microsoft"]);
/** An OAuth provider people sign in with. */
export type ProviderId = schema.Infer<typeof ProviderId>;

/** The providers the issuer signs people in through, in the order a sign-in offers them. */
export const SignInProviders = schema.object({
    /** Each provider the issuer has an application at. */
    providers: schema.array(ProviderId),
});

/** A person's answer to a request: approve it, approve its grant for this sign-in, or deny it. */
export const Decision = schema.enum(["approve", "approve-once", "deny"]);
/** A person's answer to a client's request. */
export type Decision = schema.Infer<typeof Decision>;

/** A permission a client asks a person to grant, with the sentence they read for it. */
export const RequestedPermission = Object.assign(
    GrantPermission.schema.extend({
        /** Where it applies: the space, the person's home, their account, or everywhere. */
        within: PermissionScope,
        /** What the permission lets the client do, as its declaring package describes it. */
        description: Message,
    }),
    {
        /** Read the permission a grant keeps, without how the person reads it. */
        permission(requested: RequestedPermission): GrantPermission {
            const { within: _within, description: _description, ...permission } = requested;

            return permission;
        },
    },
);
/** A permission a client asks a person to grant. */
export type RequestedPermission = schema.Infer<typeof RequestedPermission>;

/** The schema of a grant an installation asks a person for. */
const requestedGrantSchema = schema.object({
    /** The installation the grant goes to. */
    delegate: Subject,
    /** The installation's name in its space. */
    installation: schema.string().min(1),
    /** The installed package's name, such as @acme/notes. */
    package: schema.string().min(1),
    /** The address of the installation's space. */
    space: schema.string().min(1),
    /** The permissions it asks for. */
    permissions: schema.array(RequestedPermission).min(1),
});
/** The grant an installation asks a person for, signed by its space (RFC 9396). */
export const RequestedGrant = Object.assign(requestedGrantSchema, {
    /** Merge what a person granted a delegate before with what they grant it now. */
    merge(standing: RequestedGrant | null, granted: RequestedGrant): RequestedGrant {
        // keep the standing permissions the new grant does not repeat
        const kept = (standing?.permissions ?? []).filter(
            (permission) =>
                !granted.permissions.some((each) =>
                    GrantPermission.includes(
                        [RequestedPermission.permission(each)],
                        [RequestedPermission.permission(permission)],
                    ),
                ),
        );

        return { ...granted, permissions: [...kept, ...granted.permissions] };
    },
});
/** The grant an installation asks a person for. */
export type RequestedGrant = schema.Infer<typeof requestedGrantSchema>;

/** The application asking, by its client identifier and the name people read. */
const RequestClient = schema.object({
    /** The client identifier. */
    id: schema.string(),
    /** The name people read. */
    name: schema.string(),
});

/** A stored authorization request as a consent view reviews it. */
export const AuthorizationRequestReview = schema.object({
    /** The application asking. */
    client: RequestClient,
    /** The scopes it asks for. */
    scopes: schema.array(schema.enum(OAUTH_SCOPES)),
    /** Whether the person signs in again before deciding. */
    isLoginDue: schema.boolean(),
    /** The grant it asks for, its permissions marked once granted before, null for none. */
    grant: RequestedGrant.extend({
        /** The permissions, each marked once the person granted it before. */
        permissions: schema.array(RequestedPermission.extend({ isGranted: schema.boolean() })),
    }).nullable(),
});

/** A person's consent to a client, as their list of apps with access shows it. */
export const Consent = schema.object({
    /** The client the person consented to. */
    client: RequestClient,
    /** The scopes they approved. */
    scopes: schema.array(schema.string()),
    /** When they first consented. */
    grantedAt: schema.number(),
    /** The live grant the consent gave an installation, null for none. */
    grant: RequestedGrant.extend({
        /** How long it lasts: until the person removes it, or until this sign-in ends. */
        lifetime: schema.enum(["lasting", "sign-in"]),
    }).nullable(),
});

/** The signed-in person's consents, newest first. */
export const ConsentList = schema.object({
    /** The consents. */
    consents: schema.array(Consent),
});

/** A consent view's decision of a stored authorization request. */
export const AuthorizationRequestDecision = schema.object({
    /** The stored request's reference (RFC 9126). */
    requestUri: schema.string().min(1),
    /** Approve or deny. */
    decision: Decision,
});

/** Where the browser goes after a decision of an authorization request. */
export const AuthorizationRequestOutcome = schema.object({
    /** The client's redirect with its code or error. */
    location: schema.string(),
});

/** A device's sign-in as a verification view reviews it. */
export const DeviceRequestReview = schema.object({
    /** The application on the device. */
    client: RequestClient,
    /** The scopes it asks for. */
    scopes: schema.array(schema.string()),
    /** Whether the person decided it yet, and how. */
    status: schema.enum(["pending", "approved", "denied"]),
});

/** A verification view's decision of a device's sign-in. */
export const DeviceRequestDecision = schema.object({
    /** The code the device shows (RFC 8628). */
    userCode: schema.string().min(1),
    /** Approve or deny. */
    decision: Decision,
});

/** A device's sign-in once decided. */
export const DeviceRequestOutcome = schema.object({
    /** How the person decided it. */
    status: schema.enum(["approved", "denied"]),
});

/** A sign-up's check of a typed handle. */
export const HandleCheck = schema.object({
    /** The handle as typed. */
    handle: schema.string(),
    /** Whether it is free, taken, or no handle. */
    availability: HandleAvailability,
});

/** The residencies a sign-up chooses from, and the one the visitor's country suggests. */
export const ResidencyChoice = schema.object({
    /** Every residency, by its code and the name people read. */
    residencies: schema.array(schema.object({ code: schema.string(), name: schema.string() })),
    /** The residency listing the visitor's country, null for none. */
    suggested: schema.string().nullable(),
});

/** Where a signed-in person's Home answers: starting while no machine serves their home yet. */
export const HomeLocation = schema.discriminatedUnion("status", [
    schema.object({
        /** No machine answers for the person's home yet. */
        status: schema.literal("starting"),
    }),
    schema.object({
        /** A machine answers for the person's home. */
        status: schema.literal("serving"),
        /** Home's URL in the person's home space. */
        url: schema.string(),
    }),
]);
/** Where a signed-in person's Home answers. */
export type HomeLocation = schema.Infer<typeof HomeLocation>;
