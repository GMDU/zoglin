# Zoglin

A datapack pre-processor focused on power and simplicity.

It is written in Rust, so it is ✨ *blazingly fast* 🚀.

## Current Progress
Zoglin is under active development, and no releases have been
made so far.

- CLI ✅
- Namespaces ✅
- Modules ✅
- Functions (2/3)
  - Function definition ✅
  - Function calling ✅
  - Functions with parameters
- Commands (1/2)
  - Commands in functions ✅
  - Inline expressions
- Resources (3/3)
  - JSON Resources ✅
  - NBT / other resources ✅
  - Assets ✅
- Include ✅
- Proper error reporting ✅
- Import (2/4)
  - Basic imports ✅
  - Import aliasing ✅
  - Importing multiple things
  - Selective imports
- Exports
- Expressions
- Standard Library
- Conditionals
- Variables (0/3)
  - Storage variables
  - Scoreboard variables
  - Compile-time variables
- CLI
  - watch ✅

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
Resources represent non-mcfunction resources within a datapack, such as JSON files.

Resources are defined using the `res` keyword, followed by a resource type.

### JSON
For JSON resources, after the resource type, a name can be specified,
followed then by a JSON block.

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

### NBT / Other files
For file based resources, such as NBT files, a file path is specified as
a string, after the resource type.

The file path supports globbing, for passing through multiple files.
It is relative to the current file.

Example:
```
# This copies the file "nbt/structure.nbt" to
# data/namespace/structures/airship.nbt
res structures "nbt/structure.nbt"

# This copies all nbt files in nbt/airships to
# data/namespace/structures/
res structures "nbt/airships/*.nbt"

# This copies the laboratories folder to
# data/namespace/structures/laboratories
res structures "nbt/laboratories"
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

#### Optional Suffix
Because a variable's path can have the `/` character, there can
be times where a division would lead to ambiguity.

Example:
```
# Is this foo:bar/baz divided by apples, or the path foo:bar/baz/apples?
foo:bar/baz/apples
```

To fix this ambiguity, a `:` can be added to the end of the path.

Example:
```
# This is foo:bar/baz divided by apples
foo:bar/baz:/apples

# This is the path foo:bar/baz/apples
foo:bar/baz/apples
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

## Expressions
An expression can be one of:
- Function call
- Variable reference
- Literal (number, array, string, etc.)
- Unary expression
- Binary expression

### Operators
#### Assignment
- `=`
- `+=`
- `-=`
- `*=`
- `/=`
- `%=`

#### Comparison
- `==`
- `!=`
- `<`
- `<=`
- `>`
- `>=`

#### Arithmetic
- `+`
- `-` (binary or unary)
- `*`
- `/`
- `%`
- `**`

#### Logical
- `||`
- `&&`
- `!` (unary)

## Control-flow
### If-else
An if-else statement executes code based on a condition. The condition can be any expression.

The code in the block of an if statement gets run if the condition is truthy. Otherwise it goes on to the next statement in the chain.

Example:
```
if foo() + 1 {
  print("Foo")
} else if -bar() / 2 == 17 {
  print("Bar")
} else {
  print("Baz")
}
```

### While
A while loop repeatedly executes a block of code while a given condition is truthy.

Example:
```
$i = 0
while $i < 10 {
  print(i)
  i += 1
}
```

### For
For iterates over a sequence of values, and runs code for each one. If given an array, the variable is set to the next item in the array; if given a number it runs the code that number of times, and the variable increments each iteration (starting at 0).

A special range type can be used in for loops, to begin at a specific value other than 0.

```
for i in 1..10 {
  print(i) # prints 1 - 9 inclusive
}
```

### Break and continue
Sometimes you want to exit a loop before it would normally be done. This can be done using the `break` keyword.

Or, if you want to simply go to the next iteration of the loop, you can use `continue`.

Example:
```
$i = 0
while true {
  if $i == 10 {
    break
  }
  print($i) # Prints 0 - 9 inclusive
  $i += 1
}

for i in 10 {
  if i % 2 == 0 {
    continue
  }
  print(i) # prints 1, 3, 5, 7, 9
}
```

## Imports and Exports
### Imports
Functions and modules from other namespaces can be imported with the `import` keyword.
When a function/module is imported, it can be referenced by its name without the rest of the path, for the remainder of the current scope.

Example:
```
namespace example {
  module foo {
    import lib:api/foo
    
    fn do_thing() {
      foo() # Calls lib:api/foo
    }
  }
  
  fn other() {
    # The import is not in scope any more, so you must use the full path
    lib:api/foo()
  }
  
  # This allows lib:api to be referenced as api
  import lib:api
  
  fn bar() {
    api/bar()
  }
}

namespace lib {
  module api {
    export fn foo() {
      ...
    }
    
    fn bar() {
      ...
    }
  }
  
  fn other() {
    ...
  }
}
```

You can use the `as` keyword to import a module/function and reference it by a different name.

Example:
```
import lib:api # aliased to api
import lib:api as lib_api # aliased to lib_api
import lib as library

lib_api/foo()
library:api/bar()
```

You can use curly braces (`{`, `}`) to import mutiple modules/functions at once.

Example:
```
import lib:api/{foo, bar}
foo()
bar()

import lib:{api, other}
api/foo()
other()
```

When a module or namespace is imported, the compiler also automatically imports anything exported by that module/namespace (see [exports](#Exports))
If you don't want to inlcude the exports of a module, you can suffix it with a `/`.

Example:
```
import lib:api # Includes exports
import lib:api/ # Excludes exports

# api can still be used normally
api/foo()
# api cannot be used as a function, as it is explicitly a module
api() # ERROR
```

If the situation is the other way around, and you don't want the module name, but only the exports, you can use `/@`.

Example:
```
import lib:api/@

foo()
api/foo() # ERROR: only imported the exports
```

If you want to import everything from a module, including non-exported functions and modules (but excluding private ones), you can use `/*`.

Example:
```
import lib:*

api/foo()
other()
```

### Exports

Exports can be used to automatically include modules/functions when importing a module or namespace.

You can export something using the `export` keyword, followed by either a module/function definition, or the resource location of a function or module.
Everything exported from a module must not be private, and must be from within that module, or a child of it.

Example:
```
namespace main {
  import lib
  baz() # lib exports baz, so this refers to lib:api/sub/baz
  
  import lib:api
  sub/baz() # lib:api exports sub
  foo()
  bar()
}

namespace lib {
  export api/sub/baz
  
  private fn internal() {
    ...
  }
  
  export internal # ERROR: internal is private

  module api {
    export fn foo() {
      ...
    }

    export bar

    fn bar() {
      ...
    }

    export module sub {
      export :api/foo # ERROR: cannot export from a parent module
      
      fn baz() {
        ...
      }
    }
  }
}
```

## Include
Files can be included using the `include` keyword. When a file is included,
Zoglin behaves as if the contents of the target file were written in the
including file.

Include takes a single argument as a string, which is a path to
the target file, relative to the including file.
This path supports globbing, but the order of the included files is not guaranteed,
so only use this if the files are order-insensitive

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