# pincher-flux-bridge

Bridge between pincher reflex actions (from `.nail` bundles) and flux-core bytecode IR for compilation through the five-layer agent cognition stack.

## Why This Exists

Pincher produces `.nail` bundles containing *reflexes* — intent→action pairs with confidence scores. These reflexes are fast paths: pattern matches that fire without LLM involvement. But the rest of the agent stack (flux-core) speaks a different language: bytecode IR with stack operations, conditionals, and branching. This crate translates between them.

The bridge is bidirectional: `reflex_to_flux` converts high-confidence reflexes into `FluxIR` instructions (MatchIntent → ConditionalExec → Halt triples), and `flux_to_teach` converts IR back into reflexes for the teach interface. The round-trip is lossless for well-structured IR.

Confidence filtering is the key design choice: only reflexes above a threshold get compiled to IR. Low-confidence reflexes stay in pincher's teach queue, waiting for more evidence. This prevents premature compilation of unreliable patterns.

## Architecture

```text
.nail Bundle (from pincher)
├── Reflex { intent, action, confidence, invoke_count }
│
▼  reflex_to_flux(threshold)
FluxIR Instructions
├── MatchIntent("list files")           ← Pattern match
├── ConditionalExec { action, threshold } ← Guard
├── Halt                                ← End of triple
├── MatchIntent("delete temp")
├── ConditionalExec { action, threshold }
├── Halt
│
▼  flux_to_teach()
Reflexes (for teach interface)

Conversion Fidelity:
├── total_reflexes
├── converted
├── skipped_low_confidence
├── skipped_unsupported
└── fidelity_ratio = converted / total
```

### FluxIR Instructions

| Instruction | Stack Effect | Purpose |
|------------|-------------|---------|
| `Push(Trit)` | +1 | Push constant onto stack |
| `Add` | −1 | Z₃ addition of top two |
| `Mul` | −1 | Z₃ multiplication of top two |
| `Load(name)` | +1 | Load named variable |
| `Store(name)` | −1 | Store to named variable |
| `MatchIntent(pattern)` | +1 | String match (0 or 1) |
| `ConditionalExec { action, threshold }` | −1 | Execute if confidence ≥ threshold |
| `BranchIf(addr)` | −1 | Jump if top is nonzero |
| `Halt` | 0 | Stop execution |
| `Nop` | 0 | Placeholder |

The `MatchIntent → ConditionalExec → Halt` triple is the canonical reflex pattern. Each reflex compiles to exactly three instructions. The `ConditionalExec` guard ensures that compiled reflexes respect the confidence threshold at runtime.

## Usage

```rust
use pincher_flux_bridge::*;

// Create a nail bundle with reflexes
let bundle = NailBundle {
    agent_name: "assistant".into(),
    reflexes: vec![
        Reflex {
            intent: "list files".into(),
            action: "ls -la".into(),
            confidence: 0.9,
            invoke_count: 50,
        },
        Reflex {
            intent: "delete temp".into(),
            action: "rm /tmp/*".into(),
            confidence: 0.3, // Below threshold — will be skipped
            invoke_count: 2,
        },
        Reflex {
            intent: "show status".into(),
            action: "git status".into(),
            confidence: 0.85,
            invoke_count: 30,
        },
    ],
};

// Convert to Flux IR with confidence threshold 0.5
let (ir, fidelity) = reflex_to_flux(&bundle, 0.5);
assert_eq!(fidelity.converted, 2);
assert_eq!(fidelity.skipped_low_confidence, 1);
assert!((fidelity.fidelity_ratio - 0.667).abs() < 0.01);

// IR contains MatchIntent → ConditionalExec → Halt triples
assert!(ir.contains(&FluxIR::MatchIntent("list files".into())));
assert!(ir.contains(&FluxIR::MatchIntent("show status".into())));

// Convert back to reflexes (round-trip)
let reflexes = flux_to_teach(&ir);
assert_eq!(reflexes.len(), 2);
assert_eq!(reflexes[0].intent, "list files");

// Z₃ arithmetic for ternary logic in the IR
assert_eq!(z3_add(-1, -1), 1);   // Z₃ wrapping
assert_eq!(z3_add(1, 1), -1);    // Z₃ wrapping
assert_eq!(z3_mul(-1, -1), 1);   // Double negative
assert_eq!(z3_mul(0, 1), 0);     // Zero annihilates

// Perfect fidelity when all reflexes pass threshold
let perfect = NailBundle {
    agent_name: "test".into(),
    reflexes: vec![
        Reflex { intent: "a".into(), action: "b".into(), confidence: 0.95, invoke_count: 10 },
        Reflex { intent: "c".into(), action: "d".into(), confidence: 0.88, invoke_count: 5 },
    ],
};
let (_, f) = reflex_to_flux(&perfect, 0.5);
assert!((f.fidelity_ratio - 1.0).abs() < 1e-10);
```

## API Reference

### NailBundle & Reflex
- `NailBundle { agent_name, reflexes }` — A pincher bundle containing named reflexes
- `Reflex { intent, action, confidence, invoke_count }` — A single intent→action mapping with reliability metrics

### FluxIR (Enum)
- `Push(Trit)` / `Add` / `Mul` — Stack operations using Z₃ arithmetic
- `Load(String)` / `Store(String)` — Named variable access
- `MatchIntent(String)` — Pattern match against intent string, pushes 0 or 1
- `ConditionalExec { action: String, threshold: f64 }` — Execute action if top-of-stack confidence ≥ threshold
- `BranchIf(usize)` / `Halt` / `Nop` — Control flow

### Conversion Functions
- `reflex_to_flux(bundle, confidence_threshold)` → `(Vec<FluxIR>, ConversionFidelity)` — Compile reflexes above threshold into IR triples
- `flux_to_teach(instructions)` → `Vec<Reflex>` — Decompile IR back to reflexes (round-trip)

### ConversionFidelity
- `ConversionFidelity { total_reflexes, converted, skipped_low_confidence, skipped_unsupported, fidelity_ratio }` — Metrics on what made it through
- `ConversionFidelity::perfect(n)` — Construct a perfect (100%) result for n reflexes

### Z₃ Arithmetic
- `z3_add(a, b)` → `Trit` — Z₃ addition (explicit 9-case match for correctness)
- `z3_mul(a, b)` → `Trit` — Z₃ multiplication (sign product)

## The Deeper Idea

This crate sits at the boundary between two representations: pincher's reflex-based pattern matching and flux-core's stack-based bytecode. Reflexes are fast but rigid — they match exactly the patterns they were taught. Flux IR is flexible and compositional — you can combine instructions to build complex behaviors. The bridge lets reflexes *graduate* into compiled IR once they've proven reliable (high confidence, many invocations).

The confidence threshold is the gatekeeper. A reflex with 0.3 confidence and 2 invocations hasn't earned compilation — it's still experimental. A reflex with 0.9 confidence and 50 invocations has proven itself and deserves the speed of compiled IR. This is how agent cognition scales: fast paths for proven behaviors, fallback for novel ones.

The Z₃ arithmetic in FluxIR enables ternary logic operations within the bytecode. Stack-based ternary computation allows the IR to express conditions like "if the majority of voices agree, execute" — the same ternary consensus used throughout the ecosystem.

## Related Crates

- [`character-sheet`](../character-sheet) — The `.nail` bundle format that pincher produces
- [`character-encounter`](../character-encounter) — The encounter engine that runs compiled reflexes
- [`ternary-cuda-kernels`](../ternary-cuda-kernels) — GPU acceleration for ternary operations in the IR
