
# bf-jit

A just-in-time compiler for the Brainfuck programming language.
Compiles directly to x86-64 machine code (no assembly).

## System requirements

Should compile code for any x86-64 Linux system.
However it's only been tested on NixOS 23.11 (Tapir) with Linux 6.1.74.

## Install

Build from source using cargo.

```bash
$ git clone --branch main https://github.com/chellipse/bf-jit
$ cd bf-jit
$ cargo install --path .
```

Make sure ~/.cargo/bin is in your PATH.
