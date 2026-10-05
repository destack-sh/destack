# @destack/fs

Lock and read host files through operating-system APIs.

## Locks

`FileLock.acquire` waits for an exclusive operating-system lock on a file, and disposing the `FileLock` releases it.

```ts
import { FileLock } from "@destack/fs";

await using lock = await FileLock.acquire("/path/to/app.lock", { signal });
await using other = await FileLock.tryAcquire("/path/to/other.lock"); // undefined while another process holds it
```

## Lock files

`FileLock` locks the file at a path, so the lock file belongs in a trusted directory that no other process renames or deletes.

```ts
await using lock = await FileLock.acquire(join(stateDirectory, "daemon.lock"), { signal });
```

## Runtime directory

`runtimeDirectory` returns `XDG_RUNTIME_DIR`, or else a private `destack-<uid>` directory under the temporary one, refusing one another user owns, others may open or a link names.

```ts
import { runtimeDirectory } from "@destack/fs";

await using lock = await FileLock.acquire(join(await runtimeDirectory(), "app.destack.host.lock"));
```

## Files

`readOptional` reads a text file, returns `undefined` when it does not exist, and throws on any other error.

```ts
import { readOptional } from "@destack/fs";

const text = await readOptional("/path/to/installation.json"); // undefined before the first install
```

## Errors

A failed operation throws a `FileSystemError` with the operation, the path and the operating-system error code, and a caller receives it as an internal server error.

```ts
import { FileSystemError } from "@destack/fs/error";

if (error instanceof FileSystemError && error.code === "EACCES") {
    report(`cannot ${error.operation} ${error.path}`);
}
new FileSystemError("read", "/notes/today.md", "ENOENT").toServiceError(); // { code: "INTERNAL_SERVER_ERROR", … }
```
