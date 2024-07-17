# ClacJIT

A Just-In-Time compiler and interpreter for Clac. Written in Rust.

## Introduction

`clacjit` is a Just-In-Time compiler and interpreter for `clac` language.

`clac` is a command line, stack-based calculator with postfix notation.

`clacjit` generally follows the same `clac` specification used in [CMU 15-122](https://www.cs.cmu.edu/~15122/). However:

- Some clac programs will trigger error in 122-clac but not in `clacjit`.

- In jit mode, only stack underflow is checked. You will get segfault, fpe, etc. Technically, you can also access arbitrary address.

- jit only supports x64 devices.

## Usage

- Build: `cargo b -r`

- Run without jit: `clacjit <file1> <file2> <...>`

- Run with jit: `clacjit --jit <file1> <file2> <...>`

## Examples

Run my MNIST implementation in clac:

- With JIT: `cargo r ./clac/mnist.clac ./clac/mnist-main.clac --jit`

  Takes 3.84s

- Without JIT: `cargo r ./clac/mnist.clac ./clac/mnist-main.clac`

  Takes 5.14s
