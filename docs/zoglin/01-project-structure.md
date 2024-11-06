# Project Structure

## Namespaces
Namespaces are top-level elements of Zoglin, and represent a namespace
in MCFunction.

All subsequent elements in Zoglin are children of namespaces.

### Block Syntax
```zoglin
namespace example {
  ...
}
```

### Block-less Syntax
```zoglin
namespace example
# Code here is in example
...

namespace example2
# Code here is in example2
...
```

## Modules
Modules represent a folder in the generated datapack.
They can be nested within other modules.

```zoglin
module foo {
  ...

  # Modules can be nested
  module bar {
    ...
  }
}
```