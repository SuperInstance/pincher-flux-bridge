# pincher-flux-bridge

*Where reflex meets deliberation.*

---

The bridge between pincher's reflex runtime and flux-core's bytecode IR. This is the connective tissue between Layer 1 (pincher: spinal cord reflexes) and Layer 3 (flux-core: cortical deliberation) of the SuperInstance five-layer stack.

When a pincher reflex has high enough confidence — when it's proven itself reliable through repeated success — it can be compiled into Flux IR for higher-order composition. And when Flux IR produces a stable pattern, it can be converted back to pincher teach format so the spinal cord absorbs what the cortex learned.

The bridge is selective: only reflexes above a confidence threshold get promoted. The ConversionFidelity struct tracks how much information survived the crossing.

Provides: NailBundle reader, ReflexToFlux converter (confidence-gated), FluxToTeach reverse converter, ConversionFidelity tracking, Z₃ arithmetic primitives.

9 tests covering Z₃ tables, bidirectional conversion, confidence filtering, empty bundles, noise handling.

Part of [SuperInstance](https://github.com/SuperInstance/SuperInstance). Connects [pincher](https://github.com/SuperInstance/pincher) ↔ [flux-core](https://github.com/SuperInstance/flux-core).

License: MIT
