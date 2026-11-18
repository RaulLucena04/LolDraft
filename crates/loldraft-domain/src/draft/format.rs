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

    /// Formato de ranked (SoloQ): 5 baneos por equipo y una única fase de picks
    /// con el orden 1-2-2-2-2-1. Los baneos se representan alternos porque el
    /// modelo exige una secuencia, aunque en el cliente sean simultáneos.
    pub fn soloq() -> Self {
        let mut steps = Vec::with_capacity(20);
        let mut push = |side, kind| steps.push(DraftStep { side, kind });

        // Baneos: cinco por equipo, alternando desde azul.
        for index in 0..5 {
            let side = if index % 2 == 0 {
                Side::Blue
            } else {
                Side::Red
            };
            push(side, ActionKind::Ban);
        }
        for index in 0..5 {
            let side = if index % 2 == 0 {
                Side::Red
            } else {
                Side::Blue
            };
            push(side, ActionKind::Ban);
        }

        // Picks: azul, rojo, rojo, azul, azul, rojo, rojo, azul, azul, rojo.
        for side in [
            Side::Blue,
            Side::Red,
            Side::Red,
            Side::Blue,
            Side::Blue,
            Side::Red,
            Side::Red,
            Side::Blue,
            Side::Blue,
            Side::Red,
        ] {
            push(side, ActionKind::Pick);
        }

        Self {
            id: "soloq".to_string(),
            name: "SoloQ (ranked)".to_string(),
            steps,
        }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            "standard" => Some(Self::standard()),
            "soloq" => Some(Self::soloq()),
            _ => None,
        }
    }

    pub fn step(&self, index: usize) -> Option<&DraftStep> {
        self.steps.get(index)
    }

    /// Índice de acción del pick número `slot` (1-based) del lado indicado.
    /// Sirve para evaluar un pick concreto sin recorrer el draft entero.
    pub fn pick_action_index(&self, side: Side, slot: usize) -> Option<usize> {
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.kind == ActionKind::Pick && step.side == side)
            .map(|(index, _)| index)
            .nth(slot.checked_sub(1)?)
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
    fn soloq_has_five_bans_per_side_then_picks() {
        let format = DraftFormat::soloq();
        assert_eq!(format.len(), 20);
        assert_eq!(format.id, "soloq");
        assert_eq!(
            format
                .steps
                .iter()
                .filter(|step| step.kind == ActionKind::Ban)
                .count(),
            10
        );
        assert!(format.steps[..10]
            .iter()
            .all(|step| step.kind == ActionKind::Ban));
        assert!(format.steps[10..]
            .iter()
            .all(|step| step.kind == ActionKind::Pick));

        let picks: Vec<Side> = format.steps[10..].iter().map(|step| step.side).collect();
        assert_eq!(
            picks,
            vec![
                Side::Blue,
                Side::Red,
                Side::Red,
                Side::Blue,
                Side::Blue,
                Side::Red,
                Side::Red,
                Side::Blue,
                Side::Blue,
                Side::Red,
            ]
        );
    }

    #[test]
    fn formats_are_looked_up_by_id() {
        assert_eq!(
            DraftFormat::by_id("standard"),
            Some(DraftFormat::standard())
        );
        assert_eq!(DraftFormat::by_id("soloq"), Some(DraftFormat::soloq()));
        assert_eq!(DraftFormat::by_id("otro"), None);
    }

    #[test]
    fn pick_slots_map_to_action_indices() {
        let soloq = DraftFormat::soloq();
        assert_eq!(soloq.pick_action_index(Side::Blue, 1), Some(10));
        assert_eq!(soloq.pick_action_index(Side::Red, 1), Some(11));
        assert_eq!(soloq.pick_action_index(Side::Blue, 5), Some(18));
        assert_eq!(soloq.pick_action_index(Side::Red, 5), Some(19));
        assert_eq!(soloq.pick_action_index(Side::Blue, 0), None);
        assert_eq!(soloq.pick_action_index(Side::Blue, 6), None);

        let standard = DraftFormat::standard();
        assert_eq!(standard.pick_action_index(Side::Blue, 1), Some(6));
        assert_eq!(standard.pick_action_index(Side::Red, 1), Some(7));
        assert_eq!(standard.pick_action_index(Side::Blue, 5), Some(18));
        assert_eq!(standard.pick_action_index(Side::Red, 5), Some(19));
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
