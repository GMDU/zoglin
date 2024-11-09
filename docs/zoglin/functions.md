# Functions
Each function defined in Zoglin represents a `.mcfunction` file in the resulting
datapack. They must be a child of a [namespace](project-structure.md#namespaces),
and can be nested within any amount of [modules](project-structure.md#modules).

## Definition
Functions are defined with the `fn` keyword, followed by an identifier, then a list
of arguments in parentheses.

```zoglin title="main.zog"
namespace code

# No arguments
fn foo() {
  ...
}

# Storage, scoreboard and macro arguments
fn bar(a, $b, %c) {
  ...
}
```
Functions arguments can be any of the variable types, except compile-time.
Compile-time variables can only be used with compile-time functions.

See [variables](variables.md) for more explanation of argument types.

### Body
The body of a function can contain expressions, commands, function calls, and control
flow statements such as `if` or `while`.

```zoglin title="main.zog"
namespace code

fn baz($iter, message) {
  while $iter > 0 {
    $iter -= 1
    &print(message)
  }
}
```

See [statements](./statements.md) for more information.

### Return
Functions can return using a storage variable, a scoreboard variable, or
by using the vanilla return system.

To return a value, one can use the `return` keyword.

Functions return to storage by default. For example, the following function
will return the string `"Hello, World!"`

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    namespace code

    fn hello_world() {
      return "Hello, World!"
    }
    ```
=== "MCFunction (.mcfunction)"
    ```mcfunction title="hello_world.mcfunction"
    # Sets the special return storage variable to "Hello, World!"
    data modify storage code:hello_world return set value "Hello, World!"
    ```

To return a scoreboard or vanilla return value, the function name must be prefixed
by a `$` or `%` respectively.

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    # Returns a scoreboard variable
    fn $add($a, $b) {
      return $a + $b
    }

    # Returns using the vanilla return
    fn %multiply($a, $b) {
      return $a * $b
    }
    ```
=== "MCFunction (.mcfunction)"
    ```mcfunction title="add.mcfunction"
    # Sets the $return special variable
    scoreboard players operation $return code.add = $a code.add
    scoreboard players operation $return code.add += $b code.add
    ```

    ```mcfunction title="multiply.mcfunction"
    # Uses return run to return the result of the operation
    scoreboard players operation $var_0 zoglin.internal.vars = $a code.add
    return run scoreboard players operation $var_0 zoglin.internal.vars += $b code.add
    ```

## Function Calls

## Examples