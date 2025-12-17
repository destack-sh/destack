# Struct Literals

Tests for Destack struct literal formatting.

## Basic Structs

### simple struct literal

Struct literals use the type name followed by braces with field assignments.

```ds
Point { x : 1 , y : 2 }
```

Field colons have no space before and one space after.

```ds expected
Point { x: 1, y: 2 };
```

### struct literal with spread

The spread operator attaches directly to the identifier with no space.

```ds
Point { x : 1 , ... other }
```

```ds expected
Point { x: 1, ...other };
```
