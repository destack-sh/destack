# Type Hierarchy

## Subtypes

### interface subtypes

Type hierarchy should list the subtypes of the selected interface.

```ds:main.ds
interface /*type_hierarchy*/Animal {}

class Dog implements Animal {}
```

```lsp type_hierarchy_subtypes
item=main.ds:3:1-3:31
name=Dog
kind=class
selection=main.ds:3:7-3:10
```

## Supertypes

### class supertypes

Type hierarchy should list the supertypes of the selected class.

```ds:main.ds
class Base {}

class /*type_hierarchy*/Derived extends Base {}
```

```lsp type_hierarchy_supertypes
item=main.ds:1:1-1:14
name=Base
kind=class
selection=main.ds:1:7-1:11
```

