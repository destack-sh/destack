# @platform/stack

The platform Destack operates, as it declares it: its organization, its residencies, and the stacks its spaces follow.

```ts
import { HOME_STACK, PLATFORM, RESIDENCIES, RESIDENCY_STACK, SPACE_STACK } from "@platform/stack";

// the organization operating the platform, and the residencies it serves
PLATFORM.handle; // "destack"
PLATFORM.reserved.includes("support"); // true
RESIDENCIES.map((residency) => residency.code); // ["eu", "us"]

// the public registry and the platform's billing, over the `platform` vault the operator writes once
RESIDENCY_STACK.definition.installations; // { vault, forge, finance, message, notification, relay, observability }

// every account's home: Home following its latest tag, the inbox, and connections with the vault of their credentials
HOME_STACK.definition.installations; // { vault, connection, home, message, inbox, observability }

// every other space
SPACE_STACK.definition.installations; // { social, message, inbox, observability }
```
