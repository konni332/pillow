# Format

This document lays out the expected file format, for compiled Pillow bytecode programs.
A Pillow bytecode file contains a header followed by structured sections.

The format is designed with the following goals:

- Deterministic and simple parsing
- Forward compatibility via versioning
- Efficient loading in both `std` and `no_std` environments
- Extensibility for future sections(e.g. native code sections)

All multi-byte integers are encoded in little-endian byte-order.

---

## File Layout

A Pillow bytecode file is composed of the following top-level structure:

- Header
- Instruction Section
- Constant Pool Section

Sections appear in a fixed order.
Future Version may append additional sections after the constant pool.

---

## Header

The header identifies the file and provides the metadata necessary to locate each section.

| Field            | Type    | Description                                |
| ---------------- | ------- | ------------------------------------------ |
| magic            | [u8; 4] | file identifier. Must be equal "PILW"      |
| version          | u16     | file format version                        |
| flags            | u16     | reserved for future features               |
| instruction_size | u32     | size of the instruction section in bytes   |
| constants_size   | u32     | size of the constant pool section in bytes |

---

## Instruction Section

The instruction section contains the raw bytecodes executed by the VM.

Instructions are encoded sequentually with no padding.
Each instruction begins with a single byte opcode, optionally followed by operands(e.g. size for alloc instruction)

---

## Constant Pool Section

The constant pool stores the literal values referenced by the bytecode instructions.
All values are 64-bit NaN-boxed values.

See the `pillow-nan` docs to see more about the specific NaN-boxing scheme.

---
