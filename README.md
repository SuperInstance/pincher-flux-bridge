# pincher-flux-bridge

*Bridge between pincher reflexes and flux-core bytecode IR. Pincher captures intent→action pairs with confidence scores. Flux-core compiles them to portable bytecode. This bridge translates between the two.*

## Why This Exists

The five-layer architecture (open-parallel → pincher → flux-core → cuda-oxide → cudaclaw) has a translation gap: pincher speaks in terms of reflexes (stimulus→response pairs with confidence), while flux-core speaks in terms of bytecode (MOVI, ADD, MUL, JMP). This bridge is the translator.

A pincher reflex like "when temperature > 80°C, activate cooling (confidence: 0.9)" becomes FLUX bytecode that can run on any device — from the agent's local CPU to a remote GPU.

## Architecture

```
Pincher Reflex:                    Flux Bytecode:
  stimulus: temp > 80              CMP R0, 80
  response: activate_cooling       MOVI R1, 1
  confidence: 0.9                  CMPS R2, 0.9  // threshold
                                   JLT skip
                                   CALL activate_cooling
                                   skip: HALT
```

### Key Types

- **`NailBundle`** — Pincher's representation of a reflex (stimulus, response, confidence).
- **`FluxIR`** — Flux-core's intermediate representation (operations, operands, labels).
- **`Bridge`** — Bidirectional converter: NailBundle → FluxIR and FluxIR → NailBundle.
- **`ConfidenceThreshold`** — Filter: only convert reflexes above this confidence. Low-confidence reflexes are dropped.
- **`ConversionResult`** — Success with IR, or error with reason (untranslatable stimulus, unsupported response).

## Usage

```rust
use pincher_flux_bridge::*;

// Create a pincher reflex
let reflex = NailBundle::new("temp > 80", "activate_cooling", 0.9);

// Bridge to flux IR
let bridge = Bridge::new(ConfidenceThreshold(0.5));
let result = bridge.to_flux(&reflex);

match result {
    ConversionResult::Success(ir) => {
        println!("Generated {} ops", ir.operations().len());
        // ir can now be compiled through flux-core → cuda-oxide → GPU
    }
    ConversionResult::Filtered => {
        println!("Confidence too low — reflex dropped");
    }
    ConversionResult::Error(reason) => {
        println!("Can't translate: {}", reason);
    }
}

// Bidirectional: flux IR → pincher reflex
let roundtrip = bridge.to_nail(&ir).unwrap();
assert_eq!(roundtrip.stimulus, reflex.stimulus);
```

## The Deeper Idea

This bridge is where the five-layer stack stops being five separate projects and becomes one system. Without it, pincher reflexes are trapped in pincher's runtime. With it, they become portable bytecode that can run on GPUs, ESP32s, or anywhere FLUX runs.

The confidence thresholding is the interesting design decision. Not every reflex deserves compilation — low-confidence reflexes are essentially noise. By filtering at the bridge, we ensure that only well-established reflexes consume compilation and execution resources. This is the same principle as `agent-intonation`'s accuracy requirements: only well-tuned behaviors get promoted.

## Related Crates

- `pincher` — The hermit crab agent that produces reflexes
- `flux-core` — The FLUX runtime that executes the compiled IR
- `cuda-oxide` — FLUX → PTX compilation for GPU
- `intent-flux-bridge` — Similar bridge for intent-based (not reflex-based) compilation
