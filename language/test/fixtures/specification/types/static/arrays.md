# Arrays

Fixed array lengths can use static parameters and associated constants.

## lengths

### static parameters size arrays

A static parameter can appear in a fixed array length.

```ds
type Lane<comptime N: uint> = [uint8; N * 2];

declare const lane: Lane<4>;
lane satisfies [uint8; 8];
```

### associated constants size arrays

An associated constant can appear in a fixed array length.

```ds
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;
lane satisfies [uint8; 8];
```

### imported constants size arrays

An imported constant can appear in a fixed array length.

```ds:a.ds
declare function length<T, comptime N: uint>(xs: [T; N]): N;

export const RGB: [uint8; 3] = [255, 128, 0];
export const N = length(RGB);

N satisfies 3;
```

```ds:b.ds
import { N } from "./a.ds";

type Lane<comptime N: uint> = [uint8; N * 2];

declare const lane: Lane<N>;
lane satisfies [uint8; 6];
```

### imported associated constants size arrays

An imported associated constant can appear in a fixed array length.

```ds:layout.ds
export class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
}
```

```ds:main.ds
import { Segment } from "./layout.ds";

type Lane = [uint8; Segment<string>.Width];

declare const lane: Lane;
lane satisfies [uint8; 8];
```
