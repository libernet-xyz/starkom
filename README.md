# Starkom Compiler

## Overview

The Starkom language allows writing zkSTARK circuits that run on the Starkom engine.

Starkom is almost identical to [Circom](https://docs.circom.io/), with only a few differences to
account for the different arithmetization scheme and lift some unnecessary restrictions implemented
in the original language. The name "Starkom" itself means "Circom on zkSTARKs".

If you now Circom, you already know Starkom.

This crate is a reusable Rust library that also compiles cleanly to WebAssembly.
