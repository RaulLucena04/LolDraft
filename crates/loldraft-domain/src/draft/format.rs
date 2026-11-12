use crate::ids::{ActionKind, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftStep {
    pub side: Side,
    pub kind: ActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftFormat {
    pub id: String,
    pub name: String,
    pub steps: Vec<DraftStep>,
}

impl DraftFormat {
    /// Formato estándar de las grandes ligas: 6 bans, 6 picks, 4 bans, 4 picks.
    pub fn standard() -> Self {
        let mut steps = Vec::with_capacity(20);
        let mut push = |side, kind| steps.push(DraftStep { side, kind });

        // Primera fase de baneos.
        push(Side::Blue, ActionKind::Ban);
        push(Side::Red, ActionKind::Ban);
        push(Side::Red, ActionKind::Ban);
        push(Side::Blue, ActionKind::Ban);
        push(Side::Blue, ActionKind::Ban);
        push(Side::Red, ActionKind::Ban);

        // Primera fase de picks.
        push(Side::Blue, ActionKind::Pick);
        push(Side::Red, ActionKind::Pick);
        push(Side::Red, ActionKind::Pick);
        push(Side::Blue, ActionKind::Pick);
        push(Side::Blue, ActionKind::Pick);
        push(Side::Red, ActionKind::Pick);

        // Segunda fase de baneos.
        push(Side::Red, ActionKind::Ban);
        push(Side::Blue, ActionKind::Ban);
        push(Side::Blue, ActionKind::Ban);
        push(Side::Red, ActionKind::Ban);

        // Segunda fase de picks.
        push(Side::Red, ActionKind::Pick);
        push(Side::Blue, ActionKind::Pick);
        push(Side::Blue, ActionKind::Pick);
        push(Side::Red, ActionKind::Pick);

        Self {
            id: "standard".to_string(),
            name: "Estándar competitivo".to_string(),
            steps,
        }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            "standard" => Some(Self::standard()),
            _ => None,
        }
    }

    pub fn step(&self, index: usize) -> Option<&DraftStep> {
        self.steps.get(index)
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ActionKind;

    #[test]
    fn standard_has_twenty_steps() {
        assert_eq!(DraftFormat::standard().len(), 20);
    }

    #[test]
    fn standard_sequence_matches_competitive_draft() {
        let format = DraftFormat::standard();
        let expected = [
            (Side::Blue, ActionKind::Ban),
            (Side::Red, ActionKind::Ban),
            (Side::Red, ActionKind::Ban),
            (Side::Blue, ActionKind::Ban),
            (Side::Blue, ActionKind::Ban),
            (Side::Red, ActionKind::Ban),
            (Side::Blue, ActionKind::Pick),
            (Side::Red, ActionKind::Pick),
            (Side::Red, ActionKind::Pick),
            (Side::Blue, ActionKind::Pick),
            (Side::Blue, ActionKind::Pick),
            (Side::Red, ActionKind::Pick),
            (Side::Red, ActionKind::Ban),
            (Side::Blue, ActionKind::Ban),
            (Side::Blue, ActionKind::Ban),
            (Side::Red, ActionKind::Ban),
            (Side::Red, ActionKind::Pick),
            (Side::Blue, ActionKind::Pick),
            (Side::Blue, ActionKind::Pick),
            (Side::Red, ActionKind::Pick),
        ];

        for (index, (side, kind)) in expected.into_iter().enumerate() {
            let step = format.step(index).expect("step");
            assert_eq!(step.side, side, "side at {index}");
            assert_eq!(step.kind, kind, "kind at {index}");
        }
    }
}
