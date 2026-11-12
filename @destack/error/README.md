# @destack/error

Declare the errors a package's callers receive, each with a stable reason, an HTTP code and typed data.

## Overview

A package declares its error family once, throws its errors by reason, and every caller receives them as the same `Failure`, after gRPC's `ErrorInfo` (AIP-193).

- **Error family**: a package's reasons, each `<NOUN>_<CONDITION>` with the schema of its data; `of` creates an error and `is` tests one, raised here or returned by a call.
- **Reason**: what failed and its condition, such as `NOTE_NOT_FOUND`; the condition sets the HTTP code and the message's status phrase.
- **Failure**: an error as the wire sends it: code, reason, domain, message and data; `of` describes any thrown value and `error` raises a received one again.

```ts
import { defineErrorFamily, Failure } from "@destack/error";
import { schema } from "@destack/schema";

const NoteError = defineErrorFamily({
    reasons: {
        NOTE_NOT_FOUND: {},
        NOTE_BLOCKED: { data: schema.object({ holder: schema.string() }) },
    },
});

const error = NoteError.of("NOTE_BLOCKED", "cannot edit note 'Plans': 'ada' is editing it", {
    data: { holder: "ada" },
});
const failure = Failure.of(error);
const raised = Failure.error(failure);
if (NoteError.is(raised, "NOTE_BLOCKED")) {
    raised.data.holder; // "ada"
}
```

## Examples

### Write a message

A message is one lowercase English sentence for developers: what, its condition's status phrase, then why, with each supplied value quoted.

```ts
import { defineErrorFamily } from "@destack/error";

const StackError = defineErrorFamily({ reasons: { INSTALLATION_NOT_FOUND: {} } });

(name: string) => StackError.of("INSTALLATION_NOT_FOUND", `installation '${name}' not found`);
```

### Read a failed response

`Failure.read` reads the failure of a response, and `Failure.error` raises it as an error with that failure.

```ts
import { Failure } from "@destack/error";

async (response: Response) => {
    if (!response.ok) {
        throw Failure.error(await Failure.read(response));
    }
};
```

### Decide whether to retry

`Failure.isTransient` decides whether a later attempt may succeed, after AIP-194.

```ts
import { Failure } from "@destack/error";

Failure.isTransient({ code: "CONFLICT", reason: "REVISION_CONFLICT" }); // true
Failure.isTransient({ code: "TOO_MANY_REQUESTS", reason: "STORAGE_EXHAUSTED" }); // false
```

## Comparison

### gRPC ErrorInfo

Reasons, domains and data follow gRPC's `ErrorInfo` after AIP-193; three things differ.

- The code is an HTTP status name, after oRPC's codes and RFC 9110's names for the ones oRPC lacks.
- A reason's condition sets its code, so `INSTALLATION_NOT_FOUND` is a 404 everywhere.
- A family types each reason's data, so `is` narrows an error to its reason's fields.
