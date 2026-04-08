# Format

This document lays out the expected file format, for compiled Pillow bytecode programs.
A Pillow bytecode file contains a header followed by structured sections.

The format is designed with the following goals:

- Deterministic and simple parsing
- Forward compatibility via versioning
- Efficient loading in both `std` and `no_std` environments
- Extensibility for future sections(e.g. native code sections)

All multi-byte integers are encoded in little-endian byte-order.


