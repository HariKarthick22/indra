use super::operation::IndraOperation;
use super::router::OperationRouter;
use super::specialist::{
    EngineeringCalcSpecialist, GeneralChatSpecialist, InspectionAnalysisSpecialist,
    PidAnalysisSpecialist, Specialist,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct SpecialistRegistry {
    specialists: Vec<Arc<dyn Specialist>>,
}

impl Default for SpecialistRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialistRegistry {
    pub fn new() -> Self {
        Self {
            specialists: vec![
                Arc::new(GeneralChatSpecialist),
                Arc::new(InspectionAnalysisSpecialist),
                Arc::new(PidAnalysisSpecialist),
                Arc::new(EngineeringCalcSpecialist),
            ],
        }
    }

    pub fn route(&self, prompt: &str) -> (IndraOperation, Arc<dyn Specialist>) {
        let operation = OperationRouter::classify(prompt);
        let specialist = self
            .specialists
            .iter()
            .find(|s| s.owned_operations().contains(&operation))
            .cloned()
            .unwrap_or_else(|| self.specialists[0].clone());
        (operation, specialist)
    }

    pub fn get_by_name(&self, name: &str) -> Option<Arc<dyn Specialist>> {
        self.specialists.iter().find(|s| s.name() == name).cloned()
    }

    pub fn list(&self) -> &[Arc<dyn Specialist>] {
        &self.specialists
    }
}
