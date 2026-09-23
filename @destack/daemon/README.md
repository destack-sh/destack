Manage enrolled host execution, checkouts, retained account credentials and audit through authenticated services.

```sh
just run
```

```ts
import { LocalClient } from "@destack/daemon/client/local";

const local = new LocalClient();
const connection = await local.inspect();
// connection.status: "uninitialized" | "disconnected" | "connected"

const client = await local.connect();
const status = await client.status();
await client.host.rename({ name: "Workstation" });
```

```ts
const checkout = await client.checkout.register({ directory: "/absolute/path/to/repository" });
const page = await client.checkout.list({ limit: 25 });
await client.checkout.unregister({ checkoutId: checkout.id });
```

```ts
// declared operations; implementation TODOs live in server/
const logins = await client.login.list();
const signIn = await client.login.signIn.start({ requestId, issuer });
await client.login.signIn.watch({ id: signIn.id });
await client.login.signOut({ requestId, loginId });

const instance = await client.instance.start({ requestId, spaceId, deploymentId, hostEpoch });
await client.instance.watch({ instanceId: instance.id });
await client.instance.stop({ requestId, instanceId: instance.id, gracePeriodMs: 10000 });

const preview = await client.preview.start({
    requestId, checkoutId, packageDirectory: ".", spaceId, loginId,
});
await client.preview.watch({ previewId: preview.id });

await client.preview.view.open({ requestId, previewId: preview.id, view: "home", path: "/" });
await client.deployment.apply({ requestId, spaceId, deploymentId, generation });
```
