use super::*;
use crate::agents::specialists::operation::IndraOperation;

#[test]
fn test_operation_router_classification() {
    // Inspection analysis
    assert_eq!(
        OperationRouter::classify("Please summarize the NDT inspection report for P-101"),
        IndraOperation::InspectionAnalysis
    );
    assert_eq!(
        OperationRouter::classify("Check the ultrasonic wall thickness and vibration data"),
        IndraOperation::InspectionAnalysis
    );

    // P&ID analysis
    assert_eq!(
        OperationRouter::classify("Find the bypass valve on the P&ID drawing"),
        IndraOperation::PidAnalysis
    );
    assert_eq!(
        OperationRouter::classify("Review piping and instrumentation schematic"),
        IndraOperation::PidAnalysis
    );

    // Engineering calculation
    assert_eq!(
        OperationRouter::classify("Calculate the pressure drop across the pipe"),
        IndraOperation::EngineeringCalculation
    );
    assert_eq!(
        OperationRouter::classify("Perform stress calculation for flange"),
        IndraOperation::EngineeringCalculation
    );

    // Code generation
    assert_eq!(
        OperationRouter::classify("Write a python script to parse CSV data"),
        IndraOperation::CodeGeneration
    );

    // General chat
    assert_eq!(
        OperationRouter::classify("Hello, what can you help me with today?"),
        IndraOperation::GeneralChat
    );
}

#[test]
fn test_specialist_registry_routing() {
    let registry = SpecialistRegistry::new();

    let (op, specialist) = registry.route("Analyze the pump P-101 vibration report");
    assert_eq!(op, IndraOperation::InspectionAnalysis);
    assert_eq!(specialist.name(), "Inspection Analysis Specialist");
    assert!(!specialist.is_stub());

    let (op, specialist) = registry.route("Can you parse this P&ID diagram?");
    assert_eq!(op, IndraOperation::PidAnalysis);
    assert_eq!(specialist.name(), "P&ID Analysis Specialist");
    assert!(specialist.is_stub());
    assert!(specialist.stub_message().is_some());

    let (op, specialist) = registry.route("What is the capital of France?");
    assert_eq!(op, IndraOperation::GeneralChat);
    assert_eq!(specialist.name(), "General Chat Specialist");
    assert!(!specialist.is_stub());
}
