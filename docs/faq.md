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

## What is Compilation Transparency?
Compilation Transparency is the concept that language features and
elements must compile in a clear and predictable way for the programmer.

This is a guiding design principle behind many of Zoglin's core features.

For example, let's take functions. A function defined in Zoglin will always
predictably generate in a folder with the same name as it's module, and will
always generate under the namespace it was defined in.

Commands within a function can be written just as they could be written in MCFunction.

Variables are always namespaced to the function they were declared in, making them easy to
access from within commands, as predicting their path is as easy as adding together the
namespace, modules, and name of the function.

This transparency allows developers to easily be able to interoperate with the generated
code directly from other, non-Zoglin datapacks, as well as allowing the developer to
better optimise and control the generated code.