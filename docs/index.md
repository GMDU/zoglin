# Welcome to Zoglin
*The result of bringing MCFunction in to the overworld...*

**Zoglin is a little language that compiles into a datapack.**
It focuses on creating simple, clean abstractions for datapack concepts,
to allow the user to craft the datapack they desire.

Zoglin is primarily targeted to technical datapack creators. Zoglin aims to be
intuitive to an experienced datapack creator, albeit providing syntax and
features similar to those found in more traditional programming languages.

=== "Zoglin (.zog)"
    ```zoglin title="main.zog"
    namespace example {
      module foo {
        fn bar() {
          # Set 'baz' to 'Hello, World!'
          baz = "Hello, World!"

          # Set 'qux' to 123
          $qux = 123

          # Run the tellraw command
          tellraw @a "Lorem ipsum dolor sit amet"
        }
      }
    }
    ```

=== "MCFunction (.mcfunction)"
    ```mcfunction title="example:foo/bar"
    # Set 'baz' to 'Hello, World!'
    data modify storage example:foo/bar baz set value "Hello, World!"

    # Set 'qux' to 123
    scoreboard players set $qux example.foo.bar 123

    # Run the tellraw command
    tellraw @a "Lorem ipsum dolor sit amet"
    ```