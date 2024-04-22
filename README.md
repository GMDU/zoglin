# Zoglin

A datapack pre-processor focused on power and simplicity.

## Namespaces
Namespaces are defined using the `namespace` block.

These represent a namespace within the generated datapack.

Functions and resources declared within a namespace block will use
that namespace by default.

Example:
```
namespace example {
  # Generates the function 'example:foo'
  fn foo() {
  ...
  }
}
```

You can also define a namespace once, which spans the entire file.

Example:

```
namespace example

# Generates the function 'example:foo'
fn foo() {
    ...
}

# Generates the function 'example:bar'
fn bar() {
    ...
}
```

## Modules
Modules are defined using the `module` block.

These represent a subfolder within a given namespace.
Modules can be nested.

Functions and resources declared within a module block will use
that module in their path, as well as all parent modules, by default.

Example:
```
namespace example {
  module api {
    module helpers {
      # Generates the function 'example:api/helpers/foo'
      fn foo() {
      ...
      }
    }
  }
}
```

## Functions
Functions are defined with the `fn` keyword.

These represent a '.mcfunction' file in the generated datapack.

Functions can contain Zoglin code, as well as regular mcfunction commands.

Example:
```
fn slow_fall() {
  effect give @s minecraft:slow_falling 10 0 true
}

# This can then be called like:
slow_fall()

# Or it can be directly called from a command like

function namespace:module/slow_fall
```

### Tick and Load
Functions named `tick` or `load` in the root of a namespace (not inside a
module) will automatically be added to the respective function tag.

Example:
```
namespace example {
  # Function is added to data/minecraft/tags/functions/tick.json
  fn tick() {
    ...
  }
  
  # Function is added to data/minecraft/tags/functions/load.json
  fn load() {
    ...
  }
  
  module api() {
    # Function isn't added to tag, as it is in a module.
    fn load() {
      ...
    }
  }
}
```


### Functions in Modules and Namespaces
If a function is within namespace / module blocks, it will automatically
assume those namespaces / modules.

Example:
```
namespace example {
  module api {
    # Generates the function 'example:api/slow_fall'
    fn slow_fall() {
      effect give @s minecraft:slow_falling 10 0 true
    }
    
    # Within the same module, it can be called as such.
    slow_fall()
  }
  
  module foo {
    # Within a different module, but within same namespace,
    # it can be called as such (namespace is inferred).
    :api/slow_fall()
  }
}

namespace test {
  module dummy {
    # Within a different namespace, it can be called as such.
    example:api/slow_fall()
  }
}
```

## Commands
Zoglin treats mcfunction commands as first class citizens.

Regular commands can be written in code blocks in exactly the same way
one would write them in mcfunction.

Example:
```
fn reset() {
  time set day
  weather clear
}
```

#### Inline Expressions
Expressions can be inserted within a command using the following syntax.

Example:
```
execute if entity @e[type=pig] run &{kill_all_pigs()}
```

Expressions are resolved at compile-time, and the resulting code
is inserted inline, where templated.

### Explicit command syntax
If a keyword or a function name shares a name with a command, the
keyword / function name will take precedence over the command.

To avoid such conflicts, lines can be prefixed with a `/`. This tells
Zoglin to treat the line as a command, regardless of the syntax.

Example:
```
fn time() {
  ...
}

# There is now a parsing conflict with the /time command.
# It should now be prefixed with '/' in this module.

# Prefixed to resolve conflicts:
/time set day
# No prefix required:
weather clear
```

Because Zoglin does not validate commands, this can be used
to output anything to the resulting '.mcfunction'.

Example:
```
/# This is a comment in mcfunction
/this is just plain invalid mcfunction
```

#### Command Blocks
If you were to write many conflicting commands in a row,
and did not desire to prefix them all, you can use a command block.

Command blocks are opened with `/-`, and closed with `-/`.

Example:
```
/-
# Command block opened
time set day
weather clear
effect clear @s
-/
# Command block closed
```

These are not to be confused with "Command Blocks" in game, which are
completely unrelated.

## Comments
In Zoglin, any line beginning with a `#` is considered a comment.
Comments within functions get put, as comments, into the resulting mcfunction file.
Comments outside of functions are ignored completely.
Comments are also allowed at the end of lines, except after commands, because Zoglin does not process them.

Example:
```
# This comment is ignored
namespace example {
  module main { # This comment is valid, and also ignored
    fn greet() {
      # This comment will appear in mcfunction
      say hello! # This comment is invalid, and will be treated as a command
    }
  }
}
```

## Resources
Resources represent JSON configuration files within a datapack.

They can be defined using the `res` keyword. It takes a resource type,
followed by its name, then a block containing JSON.

The JSON block is compatible with JSON5, which will be converted to plain
JSON at compile time.

Example:
```
namespace example {
  module api {
    # Generates a resource at data/example/predicates/api/is_sneaking.json
    res predicates is_sneaking {
      [...]
    }
  }
}
```

If the JSON contains an object at top-level, the braces can be ignored
for the blocks own braces instead.

Example:
```
res tags/blocks air_types {
  values: [
    'minecraft:air', 'minecraft:cave_air',
    'minecraft:void_air'
  ]
}
```

## Private
Modules and functions can be marked as private using the `private` keyword.

Private modules and functions are excluded from imports, but can still
be accessed by full path.

Modules and functions can be marked private individually:
```
private module foo {
  ...
}

private fn bar() {
  ...
}
```

Or in a block:
```
private {
  module foo {
    ...
  }
  
  fn bar() {
    ...
  }
}
```

The private keyword on its own will mark anything beneath it private
until the end of the current block, namespace, or file.

Example:
```
namespace example {
  # This module is public
  module api {
    ...
  }
  
  private
  
  # This module is private
  module foo {
    ...
  }
  
  # This function is private
  fn bar() {
    ...
  }
}
```



## Variables
Zoglin provides three main variable types:
- Storage
- Scoreboard
- Compile-time

Variables need not be declared, they are automatically created
on assignment.

### Storage variables
Storage variables store data directly in data storage.

They can be assigned any NBT value.

Example:
```
# data modify storage namespace:module foo set value [1,2,3]
foo = [1,2,3]
```

Storage variables largely follow the function name syntax, but without the
ending brackets.

Usual NBT syntax also applies, such as indexing (`var[0]`), or path walking
(`foo.bar.baz`).

Example:
```
# namespace:module foo
foo

# namespace:module foo.bar
foo.bar

# namespace:module foo.values[0]
foo.values[0]

# namespace:module/foo bar
foo/bar

# namespace:api example.data
:api/example.data

# minecraft:data example
minecraft:data/example
```

### Scoreboard variables
Scoreboard variables store data in scoreboard objectives. As such,
they can only represent integer values.

Scoreboard variables are prefixed with a `$`.

Example:
```
# scoreboard players set $apples namespace.module 5
$apples = 5

# scoreboard players add $apples namespace.module.fruit 10
$fruit/apples += 10

# scoreboard players set $money namespace.api 120
$:api/money = 120

# scoreboard players set $credits minecraft.data 5
$minecraft:data/credits = 342
```

#### Customizing Player Names
To utilize selectors, or to use player names not prefixed with `$`,
custom player names can be written within square brackets.

Example:
```
# scoreboard players set @s namespace.module 12
$[@s] = 12

# scoreboard players add @a namespace.api.points 1
$:api/points/[@a] += 1
```

### Compile Time Variables
Compile time variables exist only at compile-time. These are useful for
storing static data that does not need to be accessed at run-time.

Compile-time variables are prefixed with an `&`.

They can store any data type native to Zoglin.

Example:
```
&blocks = ["minecraft:stone", "minecraft:andesite", "minecraft:granite"]
```

Same name path rules apply as with storage variables.

## Imports
Functions and modules from other namespaces can be imported with the `import` keyword.

Imports are scoped to the namespace / module they were imported to.

Example:
```
namespace example {
  module code {
    # Aliases std:string/reverse() and std:string/concat()
    # to reverse() and concat()
    import std:string/{reverse, concat}
    
    reverse("Hello")
  }
  
  :code/concat("Hello", "World")
}
```

## Include
Files can be included using the `include` keyword. When a file is included,
Zoglin behaves as if the contents of the target file were written in the
including file.

Include takes a single argument as a string, which is a path to
the target file, relative to the including file.

For example, if there was a file defining a module:
```
# src/api.zog
module api {
  fn start() {
  ...
  }
  
  fn step() {
  ...
  }
}
```

Then if a file were to include it:
```
# src/main.zog
namespace example {
  # Include a file relative to current. File extension can be omitted.
  include "./api"
  
  module test {
  ...
  }
}
```

The namespace `example` would now have both the `api` module, and the `test`
module.

## CLI
Zoglin ships with a CLI for generating and compiling projects.

To begin a new Zoglin project, simply run:

```console
$ zog init
```

This will create a `main.zog` file in the current directory. The current directory must be empty to do this.

You can also run:

```console
$ zog init <name>
```

This creates a directory called `<name>`, and creates `main.zog` within that.

To build a project, you can use:

```console
$ zog build
```

By default, this will build the `main.zog` file, and place the resulting datapack in a directory called `build`.
To change these defaults, you can use the `-f` and `-o` flags respectively.

Example:
```console
$ zog build -f other.zog -o path/to/output
```

## Data Types
### Procs
Procs are essentially blocks of code that can be passed as function
arguments.

These are useful for defining anonymous functions for callbacks, or
for storing code in a variable for later use.

Procs utilize the `->` prefix operator syntax.

Example:
```
&myproc = -> { say "My proc has run!" }
func_with_callback(->(&result) { tellraw @a "#{&result}" })
```