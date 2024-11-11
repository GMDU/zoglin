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
The [statements page](./statements.md) explains the different statements allowed inside a function body.

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
#### Manual Returns
Sometimes it is convenient, for optimisation, to use a command to return a value instead of using the return keyword.

To do so, write a command that assigns to the `return` variable.
```zoglin title="main.zog"
namespace code

# Manually return a storage variable
fn storage_example() {
  data modify storage code:storage_example return set value "Example value"
}

# Manually return a scoreboard variable
fn $scoreboard_example() {
  scoreboard players set $return code.scoreboard_example set value 123
}

# Manually return with /return
fn %vanilla_example() {
  # Use the command literal syntax to use the vanilla return
  /return 123
}
```

## Function Calls
To call a function, Zoglin provides the function call syntax. Function calls are represented as a [Resource Location](./resource-locations.md)
of the function, followed by parentheses. These parentheses can hold a list of argument [expressions](./statements.md#expressions),
which get passed to the function by assigning [variables](./variables.md) before the function is called.

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    namespace code

    fn example() {
      # Call a function in a submodule
      submodule/foo()

      # Assign to a variable
      $x = $add(1, 2)

      # Use within expressions
      $y = 10 * $add(2, 3)
    }

    fn $add(a, b) {
      return a + b
    }

    module submodule {
      fn foo () {
        tellraw @a "Foo!"
      }
    }
    ```
=== "MCFunction (.mcfunction)"
    ```mcfunction title="example.mcfunction"
    # Call a function in a submodule
    function code:submodule/foo

    # Assign to a variable
    scoreboard players set $a code.add 1
    scoreboard players set $b code.add 2
    function code:add
    scoreboard players operation $x code.example = $return code.add

    # Use within expressions
    scoreboard players set $y code.example 10
    scoreboard players set $a code.add 2
    scoreboard players set $b code.add 3
    function code:add
    scoreboard players operation $y code.example *= $return code.add
    ```

    ```mcfunction title="add.mcfunction"
    scoreboard players operation $return code.add = $a code.add
    scoreboard players operation $return code.add += $b code.add
    ```

    ```mcfunction title="submodule/foo.mcfunction"
    tellraw @a "Foo!"
    ```

!!! warning "Recursively calling a function"
    Zoglin allows you to call a function from within itself, using the function call syntax.
    However, this can cause some troubles as Zoglin does not use a stack for function arguments.

    Calling a function from within itself, that has arguments, will override whatever arguments were
    set on first call.

    ```zoglin title="main.zog"
    namespace code

    fn example(x) {
      # x is currently 3

      if x != 1 { example(1) }

      # x is now 1
    }
    ```

    For this reason, we would recommend not calling a function from within itself, or from within functions it calls.

    We recommend using the [while loop](./statements.md#while) for iteration.