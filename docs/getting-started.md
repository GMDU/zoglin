# Getting Started
This guide will take you through making your first datapack with Zoglin.

## Installation
1. Download Zoglin from our [Github Releases](https://github.com/GMDU/zoglin/releases)
2. Extract the file, and put the Zoglin binary in your PATH.

## Example: Diamond Jump
In this example, we will create a datapack that gives you a diamond
whenever you jump. This example is purposefully very simple.

### Create the Project
Navigate to your projects folder in your command line / terminal.

To create a Zoglin project, you can use the `init` command:
```bash
$ zoglin init <name>
```

We will be calling this project `diamond_jump`
```bash
$ zoglin init diamond_jump
```

This will create a folder named `diamond_jump`, containing a
`main.zog` file.
```zoglin title="main.zog"
namespace diamond_jump {
  fn tick() {

  }

  fn load() {
    tellraw @a "Loaded diamond_jump"
  }
}
```
You'll notice that the default project has a `tick` and a `load` function.

Zoglin automatically adds functions named `tick` or `load` to their corresponding
function tag, as long as they are defined in the root of a namespace.

### Let's get it Coded!
Lets start by adding a scoreboard objective to `load`. Zoglin does provide
helpers to add these, but for this example we will just use a command.

```zoglin title="main.zog"
namespace diamond_jump {
  fn tick() {}

  fn load() {
    tellraw @a "Loaded diamond_jump"

    # Add a jump tracker objective
    scoreboard objectives add diamond_jump.jumps minecraft.custom:minecraft.jump
  }
}
```
This will keep track of whenever a player jumps.

Next, we'll define a function to give us a diamond.
We'll call it `grant`.
```zoglin title="main.zog"
namespace diamond_jump {
  fn tick() {}

  fn load() {
    tellraw @a "Loaded diamond_jump"

    # Add a jump tracker objective
    scoreboard objectives add diamond_jump.jumps minecraft.custom:minecraft.jump
  }

  # Define the `grant` function
  fn grant() {
    scoreboard players reset @s diamond_jump.jumps
    give @s minecraft:diamond
  }
}
```
This is a very simple function, containing only two commands.

In Zoglin, commands are first-class citizens, meaning they can be written out
in a function, just as you would in a datapack.

Now, in the `tick` function, we will check for players with the necessary score,
and run the `grant` function as them.
```zoglin title="main.zog"
namespace diamond_jump {
  fn tick() {
    execute as @a[scores={diamond_jump.jumps=1..}] run function diamond_jump:grant
  }

  fn load() {
    tellraw @a "Loaded diamond_jump"

    # Add a jump tracker objective
    scoreboard objectives add diamond_jump.jumps minecraft.custom:minecraft.jump
  }
  
  # Define the `grant` function
  fn grant() {
    scoreboard players reset @s diamond_jump.jumps
    give @s minecraft:diamond
  }
}
```

Now, build the project, and try it out in game.


## Building / Watching
To build your Zoglin project, run:
```bash
$ zoglin build
```
Make sure you are in your project's directory.

You will likely want to use the watch command, however,
as that will watch your project for changes and automatically re-build.
```bash
$ zoglin watch
```

## Example: Sum an Array
Next we will create a project that will allow you to sum all of the numbers
in a given array. This example demonstrates some of the more powerful features
of Zoglin.

### Create the Project
Let's create another project. Open the terminal to your projects folder and run:
```bash
$ zoglin init sum
```

### Let's get it Coded!
We don't need the `tick` function for this example, so you can remove it from the
file.

Let's define the function `sum_array`. It takes one argument, `array`.
This is a [storage variable](), meaning that it is stored in data storage.

Let's start by assigning `0` to the variable `$output`.
```zoglin title="main.zog"
namespace sum {
  fn load() {
    tellraw @a "Loaded sum"
  }

  fn sum_array(array) {
    $output = 0
  }
}
```

Notice, unlike the `array` variable, `$output` has a dollar (`$`) prefix.
This makes it a [scoreboard variable](), meaning that it is stored on a
scoreboard objective. It only accepts integer values.

Next, let's iterate the array. We can use the `while` loop for that.
Add the value from the front of the array to `$output`, then remove that
value. When all values are removed, `array` will have a size of `0`, and the
loop will end.

```zoglin title="main.zog"
namespace sum {
  fn load() {
    tellraw @a "Loaded sum"
  }

  fn sum_array(array) {
    $output = 0

    while array {
      $output = $output + array[0]
      data remove storage sum:sum_array array[0]
    }

    return $output
  }
}
```

Now, let's try it out! In `load`, call the function with an array, and
store the result in `$count`. Let's replace the loaded message tellraw
with one to print out the result.

```zoglin title="main.zog"
namespace sum {
  fn load() {
    $count = sum_array([1, 2, 3, 4, 5])
    tellraw @a {"score":{"name":"$count","objective":"sum.load"}}
  }

  fn sum_array(array) {
    $output = 0

    while array {
      $output = $output + array[0]
      data remove storage sum:sum_array array[0]
    }

    return $output
  }
}
```

Now [build](#building--watching) the project, and try it out in game.
On a `/reload`, you should see the number `15` printed in the chat.