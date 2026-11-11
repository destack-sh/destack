# @destack/error

Name the call errors Destack's callers receive as gRPC's `ErrorInfo` does (AIP-193): an HTTP code, a stable `reason` within its `domain`, a message and details.

## Codes

`CALL_ERROR_STATUSES` gives each `CallErrorCode` its HTTP status: oRPC's codes, and RFC 9110's names for the statuses oRPC lacks.

```ts
import { CALL_ERROR_STATUSES, type CallErrorCode } from "@destack/error";

CALL_ERROR_STATUSES.TOO_MANY_REQUESTS; // 429
CALL_ERROR_STATUSES.MISDIRECTED_REQUEST; // 421
const code: CallErrorCode = "PAYMENT_REQUIRED"; // 402
```

## Reasons

A reason is `<NOUN>_<CONDITION>`: what failed, then one of `ERROR_CONDITIONS`, gRPC's canonical failures, which sets the HTTP code and the message's status phrase, after Kubernetes' API errors.

```ts
import { ERROR_CONDITIONS, ErrorCondition } from "@destack/error";

ERROR_CONDITIONS.NOT_FOUND; // "NOT_FOUND", 404: `<noun> '<value>' not found`
ERROR_CONDITIONS.EXISTS; // "CONFLICT", 409: `<noun> '<value>' already exists`
ERROR_CONDITIONS.CONFLICT; // "CONFLICT", 409: `cannot <verb> <noun> '<value>': <why>`
ERROR_CONDITIONS.INVALID; // "BAD_REQUEST", 400: `<noun> '<value>' is invalid: <what> must <expectation>`
ERROR_CONDITIONS.REQUIRED; // "BAD_REQUEST", 400: `<what> is required`
ERROR_CONDITIONS.UNAUTHENTICATED; // "UNAUTHORIZED", 401: `authentication required: <why>`
ERROR_CONDITIONS.DENIED; // "FORBIDDEN", 403: `<noun> '<value>' is forbidden: <who> cannot <verb>`
ERROR_CONDITIONS.BLOCKED; // "PRECONDITION_FAILED", 412: `cannot <verb> <noun> '<value>': <why>`
ERROR_CONDITIONS.EXHAUSTED; // "TOO_MANY_REQUESTS", 429: `<noun> '<value>' exceeds <limit>`
ERROR_CONDITIONS.UNSUPPORTED; // "NOT_IMPLEMENTED", 501: `<thing> is not supported`
ERROR_CONDITIONS.MOVED; // "MISDIRECTED_REQUEST", 421: `<noun> '<value>' moved to '<where>'`
ERROR_CONDITIONS.UNAVAILABLE; // "SERVICE_UNAVAILABLE", 503: `<noun> '<value>' is unavailable: <why>`
ERROR_CONDITIONS.UNREACHABLE; // "BAD_GATEWAY", 502: `could not <verb> <noun> '<value>'`
ERROR_CONDITIONS.CANCELLED; // "CLIENT_CLOSED_REQUEST", 499: `<what> was cancelled`
ERROR_CONDITIONS.BROKEN; // "INTERNAL_SERVER_ERROR", 500: `<noun> '<value>' <fact that must never hold>`
ErrorCondition.of("INSTALLATION_NOT_FOUND"); // "NOT_FOUND"
```

## Families

`defineErrorFamily` declares a package's reasons, one per thing a caller does differently, each with the schema of its data.

```ts
import { defineErrorFamily } from "@destack/error";

const NoteError = defineErrorFamily({
    reasons: {
        NOTE_NOT_FOUND: {},
        NOTE_BLOCKED: { data: schema.object({ holder: schema.string() }) },
    },
});

throw NoteError.of("NOTE_BLOCKED", "cannot edit note 'Plans': 'Ada' holds it", {
    data: { holder: "ada" },
});
if (NoteError.is(error, "NOTE_BLOCKED")) error.data.holder; // "ada", raised here or returned by a call
NoteError.reason; // the schema of the reason names
```

## Messages

A message is one English sentence for developers and logs: what, its condition's status phrase, then why; people see text localized by `reason` and `data`.

```ts
// quote each value someone supplied; leave kinds, counts and our own constants bare, as Kubernetes' `pods "nginx" not found`
throw StackError.of("INSTALLATION_NOT_FOUND", `installation '${name}' not found`);
throw ObjectError.of("OBJECT_NOT_FOUND", `${object.name} '${id}' not found`);
throw BucketError.of(
    "FILE_INVALID",
    `file '${key}' is invalid: it takes ${size} bytes, over ${MAX_FILE_BYTES}`,
);
// keep a cause as the cause, not in the text
throw MailError.of("PROVIDER_UNREACHABLE", `could not send mail to '${address}'`, { cause });
// a broken invariant as a TypeError, keeping the identifiers a developer looks up
throw new TypeError(`installation '${installationId}' manages itself through a cycle`);
// leave embedded text, JSON text and multi-line dumps bare
throw new Error(`machine 'm1' stopped after ${attempts} attempts:\n${log}`);
// never: illegal, bad, unable, failed to, please
```

## Domain errors

A `DomainError` is a package's failure that knows the `Failure` its caller receives, its `reason` a cause within the package's domain.

```ts
import { DomainError, type Failure } from "@destack/error";

class NoteError extends Error implements DomainError {
    toCallError(): Failure {
        return {
            code: "NOT_FOUND",
            reason: "NOTE_NOT_FOUND",
            domain: "@example/notes",
            message: this.message,
        };
    }
}

if (DomainError.is(error)) {
    error.toCallError(); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", message: "no note Plans" }
}
```

## Failures

`Failure` is a call error as the wire sends it: `Failure.of` describes any thrown value, one without a package's reason as HTTP's, and `Failure.error` raises a received failure again.

```ts
import { Failure } from "@destack/error";

Failure.of(NoteError.of("NOTE_NOT_FOUND", "note 'Plans' not found")); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", … }
Failure.http("BAD_GATEWAY", "bad gateway"); // { code: "BAD_GATEWAY", reason: "BAD_GATEWAY", domain: "http", … }
throw Failure.error(await Failure.read(response)); // a domain error with the failure's call error
```

`Failure.isTransient` decides whether a later attempt may succeed, after AIP-194: a lost race, a move and an outage retry, and an exhausted resource retries only when it says when.

```ts
Failure.isTransient({ code: "CONFLICT", reason: "REVISION_CONFLICT" }); // true
Failure.isTransient({
    code: "TOO_MANY_REQUESTS",
    reason: "RATE_EXHAUSTED",
    data: { retryAfter: 1000 },
}); // true
Failure.isTransient({ code: "TOO_MANY_REQUESTS", reason: "STORAGE_EXHAUSTED" }); // false
Failure.isTransient({ code: "BAD_GATEWAY", reason: "RESPONSE_UNREACHABLE" }); // false
```
