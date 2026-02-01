# Morph: The Self-Optimizing Language

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Morph is a multi-stage programming language designed to eliminate the "Two-Language Problem." It allows developers to prototype with the fluidity and speed of Python while deploying with the performance and safety of C++ or Rust.

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Morph file
./target/release/mrc run examples/hello.morph

# Check stability scores
./target/release/mrc status examples/hello.morph

# Tokenize for debugging
./target/release/mrc tokenize examples/hello.morph

# Parse and show AST
./target/release/mrc parse examples/hello.morph
```

## Example

```morph
// Hello World in Morph
proto main() {
    log("Hello, Morph!")
}

// Pipe operator for data flow
proto process_data(data) {
    let result = data 
        |> filter(x => x > 0)
        |> map(x => x * 2)
        |> sum
    result |> log
}

// Pattern matching
proto get_grade(score) {
    return match score {
        90..100 => "A"
        80..89 => "B"
        70..79 => "C"
        _ => "F"
    }
}
```

## Core Philosophy: The Morphing Lifecycle

Morph exists on a spectrum of performance stages:

| Stage | Mode | Technology | Performance Target |
|-------|------|------------|-------------------|
| **0: Draft** | proto | Tree-walk Interpreter | Prototyping (Python-equiv) |
| **1: Observe** | proto | JIT + Type Profiling | Scripting (JS-equiv) |
| **2: Refine** | Transition | Hot-path Analysis | Optimization Phase |
| **3: Solid** | solid | LLVM Native Binary | Systems (C/Rust-equiv) |

## Language Features

### 1. Intent-First Syntax

Morph prioritizes readability and developer intent:

- **Pipe Operator** (`|>`): Left-to-right data flow
- **Pattern Matching**: Expressive `match` expressions
- **Type Annotations**: Optional but powerful
- **No Traditional Loops**: `for` with guards instead of `while/break/continue`

### 2. Ghost Types

Ghost Types provide zero-cost abstractions by adding metadata that is stripped during hardening:

```morph
type Email = String <Ghost: "Regex", pattern: "^.+@.+$">

type Vertex = {
    pos: Vec3,
    uv:  Vec2
} <Ghost: "Buffer_Layout_Packed">
```

### 3. Temporal Pulse Memory (TPM)

Scoped memory management with the `claim` keyword:

```morph
proto process_data(input) {
    let items = json.parse(input)
    var output = []
    for item in items {
        let temp = item.val * 1.5
        output.push(claim temp) // Claimed to parent scope
    }
    return output
}
```

### 4. Declarative Solve Blocks

State requirements, let the compiler optimize:

```morph
solve process_images(files) {
    let images = files where extension is ".jpg"
    ensure images.size < 100mb
    return images
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `mrc run <file>` | Execute in Draft mode (Stage 0) |
| `mrc status <file>` | Check stability scores |
| `mrc harden <file>` | Compile to native binary (Stage 3) |
| `mrc build` | Build and package project |
| `mrc tokenize <file>` | Debug: show tokens |
| `mrc parse <file>` | Debug: show AST |

## Project Structure

```
morph/
├── src/
│   ├── lexer/       # Tokenizer
│   ├── parser/      # Parser
│   ├── ast/         # AST definitions
│   ├── interpreter/ # Stage 0 interpreter
│   ├── types/       # Type checker
│   └── cli/         # CLI interface
├── examples/        # Example programs
├── plans/           # Architecture docs
└── tests/           # Test suite
```

## Implementation Status

- [x] Lexer (Tokenizer)
- [x] Parser
- [x] AST
- [x] Interpreter (Stage 0: Draft)
- [x] Type Checker with Ghost Types
- [ ] JIT Compiler (Stage 1: Observe)
- [ ] Profiler (Stage 2: Refine)
- [ ] LLVM Backend (Stage 3: Solid)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Morph is inspired by:
- Julia (multi-stage compilation)
- Rust (memory safety)
- Python (ease of use)
- Haskell (type system)

---

*Morph: Prototype in minutes. Deploy at the speed of light.*
