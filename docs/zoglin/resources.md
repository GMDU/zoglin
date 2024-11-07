# Resources and Assets

Resources represent non-mcfunction resources within a datapack, such as JSON files.

Resources are defined using the `res` keyword, followed by a resource type, which determines which sub-folder that resource appears in (e.g. `loot_table`, `advancement`).

## JSON
For JSON resources, after the resource type, a name can be specified,
followed then by a JSON block.

The JSON block is compatible with JSON5, which will be converted to plain
JSON at compile time.

Example:
```zoglin
namespace example 

module api {
  # Generates a resource at data/example/predicates/api/is_sneaking.json
  res predicate is_sneaking {
    [...]
  }
}
```

If the JSON contains an object at top-level, the braces can be ignored
for the blocks own braces instead.

Example:
```zoglin
res tags/blocks air_types {
  values: [
    'minecraft:air', 'minecraft:cave_air',
    'minecraft:void_air'
  ]
}
```

## Other files
For file based resources, such as NBT files, a file path is specified as
a string, after the resource type.

The file path supports globbing, for passing through multiple files.
It is relative to the current file.

Example:
```zoglin
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

## Assets

Assets represent files in a resourcepack. They are defined in the exact same way as resources, but use the `asset` keyword instead.

```zoglin
namespace example

asset models/item my_model {
  parent: "minecraft:item/generated",
  textures: {
    layer0: "example:my_texture"
  }
}

assert textures/item "assets/items/my_texture.png"
```
