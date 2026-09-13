use super::operation::IndraOperation;

pub struct OperationRouter;

impl OperationRouter {
    pub fn classify(prompt: &str) -> IndraOperation {
        let lower = prompt.to_lowercase();

        // 1. Inspection Analysis keywords
        if lower.contains("inspection")
            || lower.contains("ndt")
            || lower.contains("non-destructive")
            || lower.contains("ultrasonic")
            || lower.contains("radiographic")
            || lower.contains("wall thickness")
            || lower.contains("corrosion")
            || lower.contains("vibration")
            || lower.contains("bearing")
            || lower.contains("p-101")
            || lower.contains("p101")
            || (lower.contains("pump")
                && (lower.contains("report")
                    || lower.contains("analysis")
                    || lower.contains("check")))
        {
            return IndraOperation::InspectionAnalysis;
        }

        // 2. P&ID Analysis keywords
        if lower.contains("p&id")
            || lower.contains("pid ")
            || lower.contains("piping and instrumentation")
            || (lower.contains("drawing")
                && (lower.contains("valve")
                    || lower.contains("line")
                    || lower.contains("pipe")
                    || lower.contains("bypass")))
            || lower.contains("schematic")
            || lower.contains("isometric")
        {
            return IndraOperation::PidAnalysis;
        }

        // 3. Engineering Calculation keywords
        if lower.contains("calculate")
            || lower.contains("stress calculation")
            || lower.contains("pressure drop")
            || lower.contains("flow rate")
            || lower.contains("head loss")
            || lower.contains("reynolds number")
            || lower.contains("wall thickness calculation")
        {
            return IndraOperation::EngineeringCalculation;
        }

        // 4. Code Generation keywords
        if lower.contains("write code")
            || lower.contains("write a script")
            || lower.contains("function ")
            || lower.contains("python script")
            || lower.contains("rust code")
            || lower.contains("implement a function")
        {
            return IndraOperation::CodeGeneration;
        }

        // Default: General Chat
        IndraOperation::GeneralChat
    }
}
