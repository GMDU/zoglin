# Variables

Variables are used to store data in Zoglin. There are two primary types of variables: Scoreboard variables and Storage variables. (There are other types: See [Function Arguments](functions.md#arguments) for macro variables, and [Compile Time](compile-time.md) for comptime variables.)

As the names suggest, Scoreboard variables store data in scoreboards, and Storage variable store data in data storages. The type of a variable is defined by a prefix to the variable name.

```zoglin
# No prefix, Storage variable
storage = 10
# `$` prefix, Scoreboard variable
$scoreboard = 20
```

Variables can be assigned to, using the `=` operators. Variables do not need to be initialised beforehand. Variables may be assigned to using their full resource location, or just by a name. If just the name is provided, the variable is namespaced to the containing function.

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    namespace example

    module foo {
      fn bar() {
        # Full path is example:foo/bar baz
        baz = "Hello, World!"

        # Full path is $qux example.foo.bar
        $qux = 123

        # Full path explicitly specified as some:custom/path var
        some:custom/path/var = 20
      }
    }
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction title="example:foo/bar"
    # Full path is example:foo/bar baz
    data modify storage example:foo/bar baz set value "Hello, World!"

    # Full path is $qux example.foo.bar
    scoreboard players set $qux example.foo.bar 123

    # Full path explicitly specified as some:custom/path var
    data modify storage some:custom/path var set value 20
    ```

Variables can be used in expressions, just like they would in other programming languages. They must be prefixed with their type, since `var` and `$var` refer to different locations, and likely different data.

??? info "Note"
    There is no way to perform mathematical operations on data storages, so any time a storage variable is used in an operation, they must first be converted to scoreboards. If you are performing many operations on a piece of data, consider using a scoreboard variable instead.

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    namespace example

    fn load() {
      a = 10
      $b = a
      c = a + $b
    }
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction title="example:load"
    data modify storage example:load a set value 10
    execute store result score $b example.load run data get storage example:load a
    execute store result score $var_0 zoglin.internal.example.vars run data get storage example:load a
    scoreboard players operation $var_0 zoglin.internal.example.vars += $b example.load
    execute store result storage example:load c int 1 run scoreboard players get $var_0 zoglin.internal.example.vars
    ```
