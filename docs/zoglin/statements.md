# Statements
Statements are the contents of a function body, and compile in to lines of MCFunction code.

## Commands
In Zoglin, most commands can be written directly as they would be in MCFunction.
They compile directly to commands within the function.

```zoglin title="main.zog"
namespace example

fn foo() {
  data modify storage example:foo/bar set value "Hello, World!"
}
```

There are a couple of exceptions, such as with `return`, where the command might conflict with a keyword.

To resolve such cases, a command literal syntax also exists, which allows commands to be written inside backticks.
```zoglin title="main.zog"
namespace example

fn foo() {
  `return 123`
}
```

The backticks syntax also has the benefit of allowing commands to span multiple lines.
```zoglin title="main.zog"
namespace example

fn foo() {
  `data modify storage foo:bar baz set value [
    1, 2, 3,
    4, 5, 6,
    7, 8, 9
  ]`
}
```

Any additional whitespace is stripped from within the command. Newlines are either stripped or replaced with space.

Spaces, however, are maintained within strings inside the command.
```zoglin title="main.zog"
namespace example

fn foo() {
  `data modify storage foo:bar baz set value "This whitespace is    maintained!"`
}
```

### Inline Expressions
Commands can also have inline expressions, allowing for dynamic content to be added to commands at
compile time.

Inline expressions are written with the syntax `&{}`, where the expression itself is written between
the curly braces.

#### Function Calls
Inline expressions can contain a function call, which compiles to `function <function path>`
=== "Zoglin (.zog)"
    ```zoglin
    execute if entity @s[tag=my.tag] run &{bar()}
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction
    execute if entity @s[tag=my.tag] run function example:bar
    ```

#### Compile-time Variables / Functions
Inline expressions can contain a compile time variable or function call,
the value of which compiles to their run-time equivalents.
=== "Zoglin (.zog)"
    ```zoglin
    &a = 123
    scoreboard players set $foo example.foo value &{&a}
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction
    scoreboard players set $foo example.foo value 123
    ```

Alternately, these values can be written directly in to commands without
requiring inline expression syntax.
=== "Zoglin (.zog)"
    ```zoglin
    &b = "Hello, World!"
    tellraw @a "&b"
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction
    tellraw @a "Hello, World!"
    ```