Host filesystem operations backed by operating-system APIs.

```ts
import { FileLock } from "@destack/fs";

await using lock = await FileLock.acquire("/path/to/app.lock", { signal });
await using available = await FileLock.tryAcquire("/path/to/other.lock");
```
