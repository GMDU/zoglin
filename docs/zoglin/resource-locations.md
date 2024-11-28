# Resource Locations

Resource Locations represent the path of a resource in Zoglin. These behave similarly
to the Resource Locations in vanilla datapacks, except for a few changes to make
using them slightly more streamlined.

## Namespace
The namespace in a Resource Location is referenced by writing the name of the
namespace, followed by a colon (`:`).

For example, in the Resource Location `foo:bar`, the namespace is `foo`.

The name before the colon can be omitted to infer the current namespace.

For example, `:bar` is equivalent to the `foo:bar` from before.
```
[foo]:bar/baz/qux
 ^ namespace
```

## Path
The path in a Resource Location represents the path to a resource within a given namespace.

For example, in the Resource Location `foo:bar/baz`, the path is `bar/baz`.

Omitting a namespace altogether means that the Resource Location infers the namespace and current module.
```
foo:[bar/baz/qux]
     ^ path
```

## Target
The target is the final element of a Resource Location path. It is primarily used by variables, and represents the name of the variable,
whereas the rest of the path represents the location.

For example, the variable `foo:bar/baz` would be equivalent to the storage `foo:bar baz`.
```
foo:bar/baz/[qux]
             ^ target
```

## Example
The following example shows different Resource Location formats, which all refer to the same path (`foo:bar/baz/qux`).

```zoglin title="main.zog"
namespace foo

module bar {
  fn baz() {
    # Full path
    foo:bar/baz/qux

    # Inferred namespace
    :bar/baz/qux

    # Inferred namespace + module
    ~/baz/qux

    # Inferred namespace + module + function
    qux
  }
}
```