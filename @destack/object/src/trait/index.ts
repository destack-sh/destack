export { recoverable } from "./recoverable.ts";
export type { ManagedTable } from "./record.ts";
export { controlled, type ControlledDefinition } from "./controlled.ts";
export { suspendable } from "./suspendable.ts";
export { fencing } from "./fencing.ts";
export { bindable, CONSUMER } from "./bindable.ts";
export {
    provisioned,
    Provisioned,
    type ProvisionedDefinition,
    type ResourceType,
    type Resource,
} from "./provisioned.ts";
export { tracked, type TrackedDefinition } from "./tracked.ts";
export { nested, ParentReference } from "./nested.ts";
export { expiring, type ExpiryRule } from "./expiring.ts";
export {
    presentable,
    Presentable,
    type PresentableDefinition,
    ACCENTS,
    Cover,
    Icon,
    Presentation,
} from "./presentable.ts";
export { ObjectSearch, searchable, type SearchableDefinition } from "./search.ts";
export { orderable, Orderable, type OrderableDefinition, type Ordering } from "./orderable.ts";
export { readGrant, shareable } from "./shareable.ts";
export { versioned } from "./versioned.ts";
export { audited, declarable, propagated, type Trait } from "./trait.ts";
