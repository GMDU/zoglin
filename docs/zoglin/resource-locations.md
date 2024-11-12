# Resource Locations

Resource Locations represent the path of a resource in Zoglin. These behave similarly
to the Resource Locations in vanilla datapacks, except for a few changes to make
using them slightly more streamlined.

The following example shows three Resource Locations, which all refer to the
same path.

## Namespace
The namespace in a Resource Location is referenced by writing the name of the
namespace, followed by a colon (`:`).

For example, in the Resource Location `foo:bar`, the namespace is `foo`.

The name before the colon can be omitted to infer the current namespace.

For example, `:bar` is equivalent to the `foo:bar` from before.

## Path

## Target

## Example
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