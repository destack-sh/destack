# @destack/error

Name the service errors Destack's callers receive: the codes, the HTTP status each declares, and the report a failure converts to.

## Codes

`SERVICE_ERROR_STATUSES` gives each code its HTTP status: HTTP reason names for HTTP meanings, Destack names for Destack's own.

```ts
import { SERVICE_ERROR_STATUSES } from "@destack/error";

SERVICE_ERROR_STATUSES.TOO_MANY_REQUESTS; // 429
SERVICE_ERROR_STATUSES.INSUFFICIENT_GRANT; // 403
```

## Domain errors

A `DomainError` is a package's failure that knows the `ServiceErrorReport` its caller receives, which a service answers without the package depending on it.

```ts
import { DomainError } from "@destack/error";

if (DomainError.is(error)) {
    error.toServiceError(); // { code: "NOT_FOUND", message: "no note-0199…" }
}
```
