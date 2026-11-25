import { PermissionReference } from "@destack/access";
import type { Clock } from "@destack/environment";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";

/** Where a moved scope is served now: its scope and the machine serving it. */
export const ScopeMove = schema.object({
    /** The moved scope. */
    scope: schema.string().min(1),
    /** The machine serving it now. */
    machine: schema.identifier("machine"),
});
/** Where a moved scope is served now. */
export type ScopeMove = schema.Infer<typeof ScopeMove>;

/** Where a scope is served within its installation: its scope and the partition serving it. */
export const PartitionRoute = schema.object({
    /** The scope. */
    scope: schema.string().min(1),
    /** The partition serving it. */
    partition: schema.string().min(1),
});
/** Where a scope is served within its installation. */
export type PartitionRoute = schema.Infer<typeof PartitionRoute>;

/** The header saying when a refused request may be sent again (RFC 9110 10.2.3). */
const RETRY_AFTER_HEADER = "retry-after";

/** The status of a server that is starting or otherwise unavailable (RFC 9110 15.6.4). */
const UNAVAILABLE_STATUS = 503;

/** The statuses of refusals that state a wait: too many requests and unavailable. */
const RETRIED_STATUSES: ReadonlySet<number> = new Set([429, 503]);

/** A wait in whole seconds, the first form Retry-After takes (RFC 9110 10.2.3). */
const SECONDS = /^\d+$/u;

/** An HTTP date in the IMF-fixdate form senders generate (RFC 9110 5.6.7). */
const HTTP_DATE = /^[A-Z][a-z]{2}, \d{2} [A-Z][a-z]{2} \d{4} \d{2}:\d{2}:\d{2} GMT$/u;

/** The milliseconds in a second, as Retry-After counts seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** A refusal's body as far as its data, which states the wait. */
const RefusalBody = schema.looseObject({ data: schema.unknown() });

/** How long until a refused call may be retried, as Retry-After states it. */
export const RetryAfter = Object.assign(
    schema.object({
        /** The wait, in milliseconds. */
        retryAfter: schema.number().nonnegative(),
    }),
    {
        /** Write a failure's wait on headers in whole seconds, nothing for data without one. */
        write(headers: Headers, data: unknown): void {
            const wait = RetryAfter.safeParse(data);
            if (wait.success) {
                const seconds = Math.ceil(wait.data.retryAfter / MILLISECONDS_PER_SECOND);
                headers.set(RETRY_AFTER_HEADER, String(seconds));
            }
        },

        /** Read the wait headers state from now, in milliseconds: whole seconds, or a date. */
        read(headers: Headers, now: number): number | undefined {
            // read whole seconds, else a valid date as the wait until it
            const header = headers.get(RETRY_AFTER_HEADER) ?? "";
            const date = HTTP_DATE.test(header) ? Date.parse(header) : Number.NaN;
            if (SECONDS.test(header)) {
                return Number(header) * MILLISECONDS_PER_SECOND;
            } else if (Number.isFinite(date)) {
                return Math.max(0, date - now);
            } else {
                return undefined;
            }
        },

        /** Read the pause a starting server asks for before a retry, absent for another answer. */
        pause(response: Response, now: number): number | undefined {
            const isStarting = response.status === UNAVAILABLE_STATUS;
            const pause = isStarting ? RetryAfter.read(response.headers, now) : undefined;

            return pause !== undefined && pause > 0 ? pause : undefined;
        },

        /** Send a request again after each pause a starting server asks for, until a deadline. */
        async wait(
            send: () => Promise<Response>,
            clock: Pick<Clock, "now" | "sleep">,
            deadline: number,
            signal?: AbortSignal,
        ): Promise<Response> {
            // send again while a 503 asks for a pause that ends before the deadline
            let response = await send();
            for (
                let pause = RetryAfter.pause(response, clock.now());
                pause !== undefined && clock.now() + pause <= deadline;
                pause = RetryAfter.pause(response, clock.now())
            ) {
                await response.body?.cancel();
                await clock.sleep(pause, signal);
                response = await send();
            }

            return response;
        },

        /** Attach the wait a refusal states in its body to its headers. */
        async attach(response: Response): Promise<Response> {
            // leave an answer of another status, or one that states its wait
            const isStated = response.headers.has(RETRY_AFTER_HEADER);
            if (!RETRIED_STATUSES.has(response.status) || isStated) {
                return response;
            }

            // read the refusal's data from its body, leaving a body that is no JSON
            const body: unknown = await response
                .clone()
                .json()
                .catch(() => undefined);
            const headers = new Headers(response.headers);
            RetryAfter.write(headers, RefusalBody.safeParse(body).data?.data);

            return new Response(response.body, {
                status: response.status,
                statusText: response.statusText,
                headers,
            });
        },
    },
);
/** How long until a refused call may be retried. */
export type RetryAfter = schema.Infer<typeof RetryAfter>;

/** A grant's refusal of a permission on an object: approval or a wider grant is required. */
export const GrantRefusal = schema.object({
    /** The permission the call needs. */
    permission: PermissionReference,
    /** The object the permission applies to. */
    object: ObjectReference,
    /** The grant that refused it. */
    grant: schema.identifier("grant"),
});
/** A grant's refusal of a permission on an object. */
export type GrantRefusal = schema.Infer<typeof GrantRefusal>;

/** A call waiting for approvals: the permission, the object, and the approvals it needs. */
export const ApprovalRefusal = schema.object({
    /** The permission the call needs. */
    permission: PermissionReference,
    /** The object the permission applies to. */
    object: ObjectReference,
    /** The approvals a change applying the call needs: its delegator's one, or a role's count. */
    approvals: schema.number().int().positive(),
    /** The principals to ask: the grant's delegator, or none for a role's count of any holders. */
    approvers: schema.array(ObjectReference),
});
/** A call waiting for approvals. */
export type ApprovalRefusal = schema.Infer<typeof ApprovalRefusal>;

/** The issues a schema finds in an input or output, each with its message and path. */
export const Issues = Object.assign(
    schema.object({
        /** Each issue's message and path. */
        issues: schema.array(
            schema.object({ message: schema.string(), path: schema.array(schema.string()) }),
        ),
    }),
    {
        /** Read a schema's issues as their messages and paths of property names. */
        of(
            issues: readonly {
                readonly message: string;
                readonly path?:
                    | readonly (PropertyKey | { readonly key: PropertyKey })[]
                    | undefined;
            }[],
        ): Issues {
            return {
                issues: issues.map((issue) => ({
                    message: issue.message,
                    path: (issue.path ?? []).map((part) =>
                        String(typeof part === "object" ? part.key : part),
                    ),
                })),
            };
        },
    },
);
/** The issues a schema finds in an input or output. */
export type Issues = schema.Infer<typeof Issues>;
