use indra::sovereign::recompute::{verify_calculation, Verdict};
use indra::sovereign::sandbox::detect_backend;

#[test]
fn agreeing_calculation_is_accepted() {
    let Ok(b) = detect_backend() else { return };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 12.566, 0.01).unwrap();
    assert_eq!(v, Verdict::Agrees);
}

#[test]
fn disagreeing_calculation_is_rejected() {
    let Ok(b) = detect_backend() else { return };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 99.0, 0.01).unwrap();
    assert!(matches!(v, Verdict::Disagrees { .. }), "a wrong number was accepted");
}
