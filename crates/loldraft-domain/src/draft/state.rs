use crate::champion::ChampionCatalog;
use crate::draft::format::{DraftFormat, DraftStep};
use crate::error::DraftError;
use crate::ids::{ActionKind, ChampionKey, DraftAction, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftState {
    pub format: DraftFormat,
    pub actions: Vec<DraftAction>,
}

impl DraftState {
    pub fn new(format: DraftFormat) -> Self {
        Self {
            format,
            actions: Vec::with_capacity(20),
        }
    }

    pub fn standard() -> Self {
        Self::new(DraftFormat::standard())
    }

    pub fn replay(format: DraftFormat, actions: Vec<DraftAction>) -> Result<Self, DraftError> {
        let mut state = Self::new(format);
        for action in actions {
            state.apply(action)?;
        }
        Ok(state)
    }

    pub fn actions(&self) -> &[DraftAction] {
        &self.actions
    }

    pub fn next_step(&self) -> Option<&DraftStep> {
        self.format.step(self.actions.len())
    }

    pub fn next_step_number(&self) -> Option<usize> {
        if self.is_complete() {
            None
        } else {
            Some(self.actions.len() + 1)
        }
    }

    pub fn current_step_index(&self) -> usize {
        self.actions.len()
    }

    pub fn is_complete(&self) -> bool {
        self.actions.len() >= self.format.steps.len()
    }

    pub fn action_for(&self, side: Side, kind: ActionKind) -> Vec<ChampionKey> {
        self.actions
            .iter()
            .filter(|action| action.side == side && action.kind == kind)
            .map(|action| action.champion_key)
            .collect()
    }

    pub fn picks(&self, side: Side) -> Vec<ChampionKey> {
        self.action_for(side, ActionKind::Pick)
    }

    pub fn bans(&self, side: Side) -> Vec<ChampionKey> {
        self.action_for(side, ActionKind::Ban)
    }

    pub fn is_used(&self, champion: ChampionKey) -> bool {
        self.actions
            .iter()
            .any(|action| action.champion_key == champion)
    }

    pub fn used_champions(&self) -> Vec<ChampionKey> {
        self.actions.iter().map(|a| a.champion_key).collect()
    }

    pub fn apply(&mut self, action: DraftAction) -> Result<(), DraftError> {
        let expected = self.next_step().copied().ok_or(DraftError::Complete)?;

        if expected.side != action.side || expected.kind != action.kind {
            return Err(DraftError::WrongStep {
                expected_side: expected.side,
                expected_kind: expected.kind,
                actual_side: action.side,
                actual_kind: action.kind,
            });
        }

        if self.is_used(action.champion_key) {
            return Err(DraftError::ChampionAlreadyUsed(action.champion_key));
        }

        self.actions.push(action);
        Ok(())
    }

    pub fn apply_in(
        &mut self,
        action: DraftAction,
        catalog: &ChampionCatalog,
    ) -> Result<(), DraftError> {
        if !catalog.contains(action.champion_key) {
            return Err(DraftError::UnknownChampion(action.champion_key));
        }
        self.apply(action)
    }

    pub fn undo(&mut self) -> Option<DraftAction> {
        self.actions.pop()
    }

    pub fn side_of_pick(&self, champion: ChampionKey) -> Option<Side> {
        self.actions
            .iter()
            .find(|action| action.kind == ActionKind::Pick && action.champion_key == champion)
            .map(|action| action.side)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DraftAction;

    fn action(side: Side, kind: ActionKind, key: u16) -> DraftAction {
        DraftAction::new(side, kind, ChampionKey(key))
    }

    #[test]
    fn applies_full_standard_draft() {
        let format = DraftFormat::standard();
        let mut state = DraftState::new(format.clone());

        for (used, step) in (1u16..).zip(format.steps.iter()) {
            state
                .apply(action(step.side, step.kind, used))
                .expect("valid action");
        }

        assert!(state.is_complete());
        assert_eq!(state.picks(Side::Blue).len(), 5);
        assert_eq!(state.picks(Side::Red).len(), 5);
        assert_eq!(state.bans(Side::Blue).len(), 5);
        assert_eq!(state.bans(Side::Red).len(), 5);
        assert!(state.next_step().is_none());
    }

    #[test]
    fn rejects_wrong_side() {
        let mut state = DraftState::standard();
        let error = state
            .apply(action(Side::Red, ActionKind::Ban, 1))
            .unwrap_err();
        assert!(matches!(error, DraftError::WrongStep { .. }));
        assert!(state.actions().is_empty());
    }

    #[test]
    fn rejects_wrong_action_kind() {
        let mut state = DraftState::standard();
        let error = state
            .apply(action(Side::Blue, ActionKind::Pick, 1))
            .unwrap_err();
        assert!(matches!(error, DraftError::WrongStep { .. }));
    }

    #[test]
    fn rejects_duplicate_champion() {
        let mut state = DraftState::standard();
        state.apply(action(Side::Blue, ActionKind::Ban, 7)).unwrap();
        let error = state
            .apply(action(Side::Red, ActionKind::Ban, 7))
            .unwrap_err();
        assert_eq!(error, DraftError::ChampionAlreadyUsed(ChampionKey(7)));
    }

    #[test]
    fn rejects_action_after_complete() {
        let format = DraftFormat::standard();
        let mut state = DraftState::new(format.clone());
        for (index, step) in format.steps.iter().enumerate() {
            state
                .apply(action(step.side, step.kind, index as u16 + 1))
                .unwrap();
        }
        let error = state
            .apply(action(Side::Blue, ActionKind::Ban, 99))
            .unwrap_err();
        assert_eq!(error, DraftError::Complete);
    }

    #[test]
    fn replay_rebuilds_state_and_validates() {
        let format = DraftFormat::standard();
        let actions = vec![
            action(Side::Blue, ActionKind::Ban, 1),
            action(Side::Red, ActionKind::Ban, 2),
        ];
        let state = DraftState::replay(format.clone(), actions.clone()).unwrap();
        assert_eq!(state.actions(), actions.as_slice());

        let bad = vec![action(Side::Red, ActionKind::Ban, 3)];
        assert!(DraftState::replay(format, bad).is_err());
    }
}
