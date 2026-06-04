//! Validate command surfaces catalog semantic warnings for entitlements.

mod integration_test_helpers;

use integration_test_helpers::TestProject;

#[test]
fn validate_warns_entitlement_default_true() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  premium_feature:
    kind: entitlement
    default: true
    owner: team-a
",
    );

    let output = project.run_command(&["validate", "--all"]);
    assert!(
        output.status.success(),
        "validate should pass with warnings, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("entitlement") && stderr.contains("default: false"),
        "expected entitlement default warning on stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("flags.premium_feature.default"),
        "expected warning path on stderr, got: {stderr}"
    );
}
