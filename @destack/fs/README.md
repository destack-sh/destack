Lock and read host files through operating-system APIs.

## Locks

A `FileLock` holds an exclusive operating-system lock on a file until it is disposed.

```ts
import { FileLock } from "@destack/fs";

await using lock = await FileLock.acquire("/path/to/app.lock", { signal });
await using other = await FileLock.tryAcquire("/path/to/other.lock"); // undefined while another process holds it
```

A lock file belongs in a trusted directory, and its path stays intact while locks may exist.

## Files

`readOptional` reads a text file, or nothing when it does not exist, and fails on any other error.

```ts
import { readOptional } from "@destack/fs";

const text = await readOptional("/path/to/installation.json"); // undefined before the first install
```

## Errors

A failed operation throws `FileSystemError` with its operation, path and operating-system code.

```ts
import { FileSystemError } from "@destack/fs/error";

if (error instanceof FileSystemError && error.code === "EACCES") {
    report(`cannot ${error.operation} ${error.path}`);
}
```
