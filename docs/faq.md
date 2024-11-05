# FAQ

## Why the name Zoglin?
In Minecraft, the command bar sorts functions in the autocomplete in alphabetical
order. This has lead to a convention of datapack developers putting their private
functions in a namespace marked as `zzz` or `z_priv`. We didn't like this
convention, but wished to have our generated functions be listed at the bottom.

So we picked a name that started with a `z`, to achieve this without using an
ugly namespace. The name Zoglin was unused in this niche, and started with a
`z` so we ended up choosing it.

## Why use Zoglin? Why not use another precompiler?
Zoglin caters towards making technical datapacks. It has a focus on project
organisation, compilation transparency and flexibility.

Zoglin provides powerful core features for managing project structure. Namespaces and modules allow developers to express the folder structure of their
datapack, whilst still maintaining the flexibility of being able to define a
datapack in a single file, or split it up as they choose.

As well, Zoglin aims to provide clean abstractions above MCFunction, that compile
in a way that is transparent to the developer. This concept of compilation
transparency is a fundamental design principle guiding the development of Zoglin,
as we understand that technical datapacks often require using MCFunction in
unintended ways.