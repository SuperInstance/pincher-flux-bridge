//! # pincher-flux-bridge
//!
//! Bridges pincher reflexes (intent→action pairs with confidence) to flux-core
//! bytecode IR for compilation through the five-layer stack.

/// A single trit value.
pub type Trit = i8;

/// A reflex from pincher's .nail bundle.
#[derive(Debug, Clone)]
pub struct Reflex {
    pub intent: String,
    pub action: String,
    pub confidence: f64,
    pub invoke_count: usize,
}

/// A .nail bundle containing reflexes.
#[derive(Debug, Clone)]
pub struct NailBundle {
    pub agent_name: String,
    pub reflexes: Vec<Reflex>,
}

/// Flux IR instructions — the bytecode for agent cognition.
#[derive(Debug, Clone, PartialEq)]
pub enum FluxIR {
    /// Push a constant trit value onto the stack.
    Push(Trit),
    /// Add top two stack values (Z₃ addition).
    Add,
    /// Multiply top two stack values (Z₃ multiplication).
    Mul,
    /// Load a named variable.
    Load(String),
    /// Store to a named variable.
    Store(String),
    /// Match intent against pattern (string comparison).
    MatchIntent(String),
    /// Execute action if confidence above threshold.
    ConditionalExec { action: String, threshold: f64 },
    /// Branch if top of stack is nonzero.
    BranchIf(usize),
    /// Halt execution.
    Halt,
    /// No-op (placeholder during conversion).
    Nop,
}

/// How much information survived the reflex→IR conversion.
#[derive(Debug, Clone)]
pub struct ConversionFidelity {
    pub total_reflexes: usize,
    pub converted: usize,
    pub skipped_low_confidence: usize,
    pub skipped_unsupported: usize,
    pub fidelity_ratio: f64,
}

impl ConversionFidelity {
    pub fn perfect(n: usize) -> Self {
        Self { total_reflexes: n, converted: n, skipped_low_confidence: 0, skipped_unsupported: 0, fidelity_ratio: 1.0 }
    }
}

/// Convert high-confidence pincher reflexes to Flux IR.
pub fn reflex_to_flux(bundle: &NailBundle, confidence_threshold: f64) -> (Vec<FluxIR>, ConversionFidelity) {
    let mut instructions = Vec::new();
    let mut converted = 0;
    let mut skipped_low = 0;
    let mut skipped_unsupported = 0;

    for reflex in &bundle.reflexes {
        if reflex.confidence < confidence_threshold {
            skipped_low += 1;
            continue;
        }
        // Convert: match intent → conditional exec → halt cycle
        instructions.push(FluxIR::MatchIntent(reflex.intent.clone()));
        instructions.push(FluxIR::ConditionalExec {
            action: reflex.action.clone(),
            threshold: confidence_threshold,
        });
        instructions.push(FluxIR::Halt);
        converted += 1;
    }

    let total = bundle.reflexes.len();
    let fidelity = if total > 0 { converted as f64 / total as f64 } else { 1.0 };

    (instructions, ConversionFidelity {
        total_reflexes: total,
        converted,
        skipped_low_confidence: skipped_low,
        skipped_unsupported,
        fidelity_ratio: fidelity,
    })
}

/// Convert Flux IR back to pincher teach format.
pub fn flux_to_teach(instructions: &[FluxIR]) -> Vec<Reflex> {
    let mut reflexes = Vec::new();
    let mut i = 0;
    while i + 2 < instructions.len() {
        if let (FluxIR::MatchIntent(intent), FluxIR::ConditionalExec { action, threshold }, FluxIR::Halt) =
            (&instructions[i], &instructions[i + 1], &instructions[i + 2])
        {
            reflexes.push(Reflex {
                intent: intent.clone(),
                action: action.clone(),
                confidence: *threshold,
                invoke_count: 0,
            });
            i += 3;
        } else {
            i += 1;
        }
    }
    reflexes
}

/// Z₃ addition using explicit match arms.
pub fn z3_add(a: Trit, b: Trit) -> Trit {
    match (a, b) {
        (-1, -1) => 1,
        (-1, 0) => -1,
        (-1, 1) => 0,
        (0, -1) => -1,
        (0, 0) => 0,
        (0, 1) => 1,
        (1, -1) => 0,
        (1, 0) => 1,
        (1, 1) => -1,
        _ => 0,
    }
}

/// Z₃ multiplication.
pub fn z3_mul(a: Trit, b: Trit) -> Trit {
    match (a, b) {
        (0, _) | (_, 0) => 0,
        (-1, -1) | (1, 1) => 1,
        (-1, 1) | (1, -1) => -1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z3_add_table() {
        assert_eq!(z3_add(-1, -1), 1);
        assert_eq!(z3_add(-1, 0), -1);
        assert_eq!(z3_add(-1, 1), 0);
        assert_eq!(z3_add(0, 0), 0);
        assert_eq!(z3_add(0, 1), 1);
        assert_eq!(z3_add(1, 1), -1);
    }

    #[test]
    fn test_z3_mul_table() {
        assert_eq!(z3_mul(-1, -1), 1);
        assert_eq!(z3_mul(-1, 1), -1);
        assert_eq!(z3_mul(0, 1), 0);
        assert_eq!(z3_mul(1, 1), 1);
    }

    #[test]
    fn test_reflex_to_flux_basic() {
        let bundle = NailBundle {
            agent_name: "test".into(),
            reflexes: vec![
                Reflex { intent: "list files".into(), action: "ls".into(), confidence: 0.9, invoke_count: 50 },
                Reflex { intent: "delete temp".into(), action: "rm /tmp/*".into(), confidence: 0.3, invoke_count: 2 },
            ],
        };
        let (ir, fidelity) = reflex_to_flux(&bundle, 0.5);
        assert_eq!(fidelity.converted, 1);
        assert_eq!(fidelity.skipped_low_confidence, 1);
        assert!(ir.contains(&FluxIR::MatchIntent("list files".into())));
    }

    #[test]
    fn test_reflex_to_flux_all_pass() {
        let bundle = NailBundle {
            agent_name: "test".into(),
            reflexes: vec![
                Reflex { intent: "a".into(), action: "b".into(), confidence: 0.95, invoke_count: 10 },
                Reflex { intent: "c".into(), action: "d".into(), confidence: 0.88, invoke_count: 5 },
            ],
        };
        let (_, fidelity) = reflex_to_flux(&bundle, 0.5);
        assert_eq!(fidelity.converted, 2);
        assert!((fidelity.fidelity_ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flux_to_teach_roundtrip() {
        let bundle = NailBundle {
            agent_name: "test".into(),
            reflexes: vec![
                Reflex { intent: "list".into(), action: "ls".into(), confidence: 0.9, invoke_count: 10 },
            ],
        };
        let (ir, _) = reflex_to_flux(&bundle, 0.5);
        let reflexes = flux_to_teach(&ir);
        assert_eq!(reflexes.len(), 1);
        assert_eq!(reflexes[0].intent, "list");
        assert_eq!(reflexes[0].action, "ls");
    }

    #[test]
    fn test_flux_to_teach_ignores_noise() {
        let ir = vec![
            FluxIR::Push(1),
            FluxIR::Nop,
            FluxIR::Halt,
        ];
        let reflexes = flux_to_teach(&ir);
        assert!(reflexes.is_empty());
    }

    #[test]
    fn test_fidelity_perfect() {
        let f = ConversionFidelity::perfect(5);
        assert_eq!(f.converted, 5);
        assert!((f.fidelity_ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_empty_bundle() {
        let bundle = NailBundle { agent_name: "empty".into(), reflexes: vec![] };
        let (ir, fidelity) = reflex_to_flux(&bundle, 0.5);
        assert!(ir.is_empty());
        assert!((fidelity.fidelity_ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_confidence_threshold_filtering() {
        let bundle = NailBundle {
            agent_name: "test".into(),
            reflexes: vec![
                Reflex { intent: "a".into(), action: "a".into(), confidence: 0.6, invoke_count: 1 },
                Reflex { intent: "b".into(), action: "b".into(), confidence: 0.4, invoke_count: 1 },
                Reflex { intent: "c".into(), action: "c".into(), confidence: 0.8, invoke_count: 1 },
            ],
        };
        let (_, fidelity) = reflex_to_flux(&bundle, 0.5);
        assert_eq!(fidelity.converted, 2);
        assert_eq!(fidelity.skipped_low_confidence, 1);
    }
}
