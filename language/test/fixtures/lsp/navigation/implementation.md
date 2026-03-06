# Implementation

## Interface members

### Implementing methods

Goto implementation should return the methods that implement the selected interface member.

```ds:main.ds
interface /*impl_use*/Drawable {
   draw(): void;
}

class [|Circle|] implements Drawable {
   draw(): void {}
}

struct [|Rectangle|] implements Drawable {
   draw(): void {}
}
```

