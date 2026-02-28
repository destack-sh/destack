# Associated Comptime Constants: Type Operators

## type-operators

### service profile composes batching with mapped envelopes

> Associated comptime constants can compose with mapped associated types for API pipelines.
> Couples a value projection (`BatchSize`) with a mapped associated alias in one owner contract.
> The assertions pin both projections after substitution on a concrete implementor.

```ds
interface ServiceProfile<Request, Response> {
    comptime const BatchSize: number = 64;

    type RequestBatch = Request[this.BatchSize];
    type ResponseEnvelope = { [K in keyof Response]: Response[K] | null };
}

class UserLookup implements ServiceProfile<{ id: int32 }, { name: string, active: boolean }> {}

// mapped response shape should preserve nullable projection per field
declare const batch: UserLookup.RequestBatch;
batch satisfies { id: int32 }[64 as comptime];

// projected member should reflect substituted operator results
declare const envelope: UserLookup.ResponseEnvelope;
envelope satisfies { name: string | null, active: boolean | null };
```

### policy defaults can combine conditional and mapped associated aliases

> Conditional associated comptime constants and mapped aliases can model policy normalized config shapes.
> Uses a conditional associated constant and then feeds it into both array-shape and mapped-type aliases.
> The resulting projections confirm conditional evaluation and mapped normalization run on substituted inputs.

```ds
interface RetryPolicy<Config> {
    comptime const RetryBudget: number = Config extends { critical: true } ? 10 : 3;

    type BudgetWindow = uint16[this.RetryBudget];
    type Flags = { [K in keyof Config]: Config[K] extends boolean ? 1 : 0 };
}

class CriticalPolicy implements RetryPolicy<{ critical: true, jitter: false, telemetry: true }> {}

// conditional budget should pick the critical branch before projection
declare const budget: CriticalPolicy.BudgetWindow;
budget satisfies uint16[10];

// projected member should reflect substituted operator results
declare const flags: CriticalPolicy.Flags;
flags satisfies { critical: 1, jitter: 1, telemetry: 1 };
```

### inferred element aliases can drive associated comptime widths

> Associated type aliases with infer can feed associated comptime constants and projections.
> Derives an associated type with `infer`, then reuses that result in an associated comptime expression.
> The lane projection must reflect the inferred element type after full owner substitution.

```ds
newtype Vector<T, comptime N: int> = T;

interface BatchProfile<Sample> {
    type Element = Sample extends (infer Item)[] ? Item : Sample;
    comptime const LaneWidth: int = this.Element extends float32 ? 16 : 4;
    type Lane = Vector<this.Element, this.LaneWidth>;
}

class FloatBatches implements BatchProfile<float32[]> {}

// infer based element selection should feed lane width and alias projection
declare const lane: FloatBatches.Lane;
lane satisfies Vector<float32, 16>;
```

### unresolved value projection still rejects generic runtime paths

> Rich associated comptime expressions are rejected in value position when unresolved.
> Leaves a projection dependent on an unresolved generic owner in value position.
> The fixture ensures value-space access is gated by compile-time resolvability.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    comptime const SegmentRows: number = SegmentBytes / 256;
}

function unresolved<Row>(): number {
    SegmentPlan<Row>.SegmentRows
}
```

- contains: resolvable

### associated comptime defaults can feed mapped envelopes with fixed windows

> Associated comptime defaults can feed mapped envelopes that carry fixed windows.
> The mapped result should use the specialized comptime width in every field.

```ds
interface WindowedState<State> {
    comptime const Width: number = 2;
    type Window = uint8[this.Width];
    type Envelope = { [K in keyof State]: [State[K], this.Window] };
}

class RuntimeState implements WindowedState<{ warm: boolean, retries: int32 }> {}

declare const envelope: RuntimeState.Envelope;
envelope satisfies { warm: [boolean, uint8[2]], retries: [int32, uint8[2]] };
```

### conditional associated comptime defaults can shape mapped packet aliases

> Conditional associated comptime defaults can shape mapped packet aliases.
> Branch selection should happen before mapped alias instantiation.

```ds
interface PacketShape<Row> {
    comptime const Width: number = Row extends string ? 4 : 2;
    type Packet = { [K in "head" | "tail"]: Row[this.Width] };
}

class TextPacket implements PacketShape<string> {}

declare const packet: TextPacket.Packet;

declare const head: TextPacket.Packet["head"];
head satisfies string[4 as comptime];

declare const tail: TextPacket.Packet["tail"];
tail satisfies string[4 as comptime];
```
