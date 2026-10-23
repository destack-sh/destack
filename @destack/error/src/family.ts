import { type JsonValue, present, schema } from "@destack/schema";
import type { CallErrorCode, DomainError } from "./error.ts";
import { Failure } from "./failure.ts";

/** A reason a family declares: the call error code it answers and the schema of its data. */
export interface ReasonDeclaration {
    /** The call error code a caller receives, such as NOT_FOUND. */
    readonly code: CallErrorCode;
    /** The schema of the structured details the failure carries, none when absent. */
    readonly data?: schema.Schema<JsonValue>;
}

/** The reasons a family declares, by name. */
export type ReasonDeclarations = Readonly<Record<string, ReasonDeclaration>>;

/** The data a declared reason takes, as its schema reads it, absent without a data schema. */
export type ReasonData<Declaration extends ReasonDeclaration> =
    Declaration["data"] extends schema.Schema<JsonValue>
        ? schema.Input<Declaration["data"]>
        : undefined;

/** The data a failure of a declared reason carries, as its schema parsed it. */
export type ReasonDetail<Declaration extends ReasonDeclaration> =
    Declaration["data"] extends schema.Schema<JsonValue>
        ? schema.Infer<Declaration["data"]>
        : undefined;

/** The options of a failure, required with the data any of its reasons needs. */
export type FailureOptions<Data> = [Exclude<Data, undefined>] extends [never]
    ? [options?: ErrorOptions & { readonly data?: never }]
    : [options: ErrorOptions & { readonly data: Exclude<Data, undefined> }];

/** A reason as the manifest lists it: its domain, name and call error code (AIP-193). */
export const ReasonDescription = Failure.pick({ domain: true, reason: true, code: true });
/** A reason as the manifest lists it. */
export type ReasonDescription = schema.Infer<typeof ReasonDescription>;

/** A failure of one reason a family declares, as a caller receives it. */
export class FamilyError<
    Reason extends string = string,
    Data extends JsonValue | undefined = JsonValue | undefined,
>
    extends Error
    implements DomainError
{
    /** The package whose reason it is. */
    readonly domain: string;
    /** The failure's stable cause within its package. */
    readonly reason: Reason;
    /** The call error code a caller receives. */
    readonly code: CallErrorCode;
    /** The structured details, checked against the reason's schema, absent without one. */
    readonly data: Data;

    /** Create a failure of a family's reason. */
    constructor(
        failure: {
            readonly domain: string;
            readonly reason: Reason;
            readonly code: CallErrorCode;
            readonly data: Data;
        },
        message: string,
        options?: ErrorOptions,
    ) {
        // keep the message and cause, and the failure's domain, reason, code and data
        super(message, options);
        this.name = "FamilyError";
        this.domain = failure.domain;
        this.reason = failure.reason;
        this.code = failure.code;
        this.data = failure.data;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        const failure = {
            code: this.code,
            reason: this.reason,
            domain: this.domain,
            message: this.message,
        };

        return this.data === undefined ? failure : { ...failure, data: this.data };
    }
}

/** A package's errors: one declaration of every reason it fails with, read by its manifest. */
export class ErrorFamily<Declared extends ReasonDeclarations> {
    /** The package whose reasons they are. */
    readonly domain: string;
    /** The declared reasons, by name. */
    readonly reasons: Declared;

    /** Create a family of a package's declared reasons. */
    constructor(domain: string, reasons: Declared) {
        this.domain = domain;
        this.reasons = reasons;
    }

    /** Create a failure of a reason, with the data its schema checks when it declares one. */
    of<const Reason extends keyof Declared & string>(
        reason: Reason,
        message: string,
        ...[options]: FailureOptions<ReasonData<Declared[Reason]>>
    ): FamilyError<Reason> {
        // check the data against the reason's schema
        const declared = present<ReasonDeclaration>(this.reasons[reason], `the reason ${reason}`);
        const data = declared.data?.parse(options?.data);

        return new FamilyError(
            { domain: this.domain, reason, code: declared.code, data },
            message,
            options?.cause === undefined ? undefined : { cause: options.cause },
        );
    }

    /** Report whether a value is a failure of this family, of one reason when given. */
    is<const Reason extends keyof Declared & string = keyof Declared & string>(
        value: unknown,
        reason?: Reason,
    ): value is FamilyError<Reason, ReasonDetail<Declared[Reason]>> {
        // match the family's domain, then one of its reasons or the given one
        if (!(value instanceof FamilyError) || value.domain !== this.domain) {
            return false;
        }
        const failed = value.toCallError().reason;

        return reason === undefined ? Object.hasOwn(this.reasons, failed) : failed === reason;
    }

    /** Describe the family's reasons as the manifest lists them. */
    describe(): ReasonDescription[] {
        return Object.entries(this.reasons).map(([reason, declared]) => ({
            domain: this.domain,
            reason,
            code: declared.code,
        }));
    }
}

/** The reasons a family declares. */
export type ReasonOf<Family> =
    Family extends ErrorFamily<infer Declared> ? keyof Declared & string : never;

/** Declare a package's error family: each reason's call error code and data schema. */
export function defineErrorFamily<const Declared extends ReasonDeclarations>(
    owner: { readonly name: string },
    reasons: Declared,
): ErrorFamily<Declared> {
    return new ErrorFamily(owner.name, reasons);
}
