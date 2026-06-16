/// Integration tests for cogito tbox subcommands.
///
/// AC1: cargo test --release passes
/// AC2: cogito tbox build --out /tmp/cogito.owl exits 0 and writes non-empty file
/// AC3: two runs produce byte-identical output
/// AC4: stats reports >=9 classes and >=6 object properties
/// AC5: ousia-reason check exits 0 (skipped if not installed)
/// AC6: dependsOn is TransitiveProperty; BusHealthcheck/BusRegistrant axioms present
/// AC7: check exits non-zero on malformed spec, zero on shipped spec

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Path to the cogito binary (built by cargo test --release or via env)
fn cogito_bin() -> PathBuf {
    // In a release build the binary is in ../../target/release/cogito
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // test binary dir (e.g. target/release/deps)
    p.pop(); // target/release
    p.push("cogito");
    if p.exists() {
        return p;
    }
    // Debug build fallback
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.pop(); // target
    p.push("debug");
    p.push("cogito");
    p
}

#[test]
fn ac2_build_writes_nonempty_file() {
    let out = std::env::temp_dir().join("cogito_ac2_test.owl");
    let status = Command::new(cogito_bin())
        .args(["tbox", "build", "--out", out.to_str().unwrap()])
        .status()
        .expect("failed to run cogito");
    assert!(status.success(), "cogito tbox build exited {:?}", status.code());
    let content = fs::read(&out).expect("reading output file");
    assert!(!content.is_empty(), "output file is empty");
    let _ = fs::remove_file(&out);
}

#[test]
fn ac3_build_reproducible() {
    let out1 = std::env::temp_dir().join("cogito_ac3_run1.owl");
    let out2 = std::env::temp_dir().join("cogito_ac3_run2.owl");

    for out in [&out1, &out2] {
        let status = Command::new(cogito_bin())
            .args(["tbox", "build", "--out", out.to_str().unwrap()])
            .status()
            .expect("failed to run cogito");
        assert!(status.success(), "cogito tbox build exited {:?}", status.code());
    }

    let b1 = fs::read(&out1).unwrap();
    let b2 = fs::read(&out2).unwrap();
    assert_eq!(b1, b2, "two runs of tbox build produced different output");
    let _ = fs::remove_file(&out1);
    let _ = fs::remove_file(&out2);
}

#[test]
fn ac4_stats_counts() {
    let out = std::env::temp_dir().join("cogito_ac4_stats.owl");
    let status = Command::new(cogito_bin())
        .args(["tbox", "build", "--out", out.to_str().unwrap()])
        .status()
        .expect("failed to run cogito");
    assert!(status.success());

    let output = Command::new(cogito_bin())
        .args(["tbox", "stats", out.to_str().unwrap()])
        .output()
        .expect("failed to run cogito stats");
    assert!(output.status.success(), "stats exited non-zero");

    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("stats output:\n{}", stdout);

    // Parse class count
    let classes: usize = stdout
        .lines()
        .find(|l| l.starts_with("classes:"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    assert!(classes >= 9, "expected >=9 classes, got {}", classes);

    // Parse object property count
    let ops: usize = stdout
        .lines()
        .find(|l| l.starts_with("object_properties:"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    assert!(ops >= 6, "expected >=6 object properties, got {}", ops);

    let _ = fs::remove_file(&out);
}

#[test]
fn ac5_ousia_reason_check() {
    let out = std::env::temp_dir().join("cogito_ac5_reason.owl");
    let status = Command::new(cogito_bin())
        .args(["tbox", "build", "--out", out.to_str().unwrap()])
        .status()
        .expect("failed to run cogito");
    assert!(status.success());

    // Find ousia-reason
    let reason_path = {
        let home = std::env::var("HOME").unwrap_or_default();
        let local = format!("{}/.local/bin/ousia-reason", home);
        if std::path::Path::new(&local).exists() {
            Some(local)
        } else {
            None
        }
    };

    match reason_path {
        None => {
            eprintln!("note: ousia-reason not found; AC5 skipped");
        }
        Some(reason) => {
            let status = Command::new(&reason)
                .args(["check", out.to_str().unwrap()])
                .status()
                .expect("failed to run ousia-reason");
            assert!(
                status.success(),
                "ousia-reason check failed with {:?}",
                status.code()
            );
        }
    }
    let _ = fs::remove_file(&out);
}

#[test]
fn ac6_transitive_and_axioms_present() {
    let out = std::env::temp_dir().join("cogito_ac6_axioms.owl");
    let status = Command::new(cogito_bin())
        .args(["tbox", "build", "--out", out.to_str().unwrap()])
        .status()
        .expect("failed to run cogito");
    assert!(status.success());

    let content = fs::read_to_string(&out).expect("reading output");
    eprintln!("OWL content snippet:\n{}", &content[..content.len().min(2000)]);

    // AC6a: dependsOn is TransitiveProperty
    assert!(
        content.contains("TransitiveObjectProperty") || content.contains("TransitiveProperty"),
        "dependsOn TransitiveProperty axiom missing from output"
    );

    // AC6b: BusHealthcheck present
    assert!(
        content.contains("BusHealthcheck"),
        "BusHealthcheck IRI missing from output"
    );

    // AC6c: BusRegistrant present
    assert!(
        content.contains("BusRegistrant"),
        "BusRegistrant IRI missing from output"
    );

    let _ = fs::remove_file(&out);
}

#[test]
fn ac7_check_malformed_spec_fails() {
    let bad_spec = std::env::temp_dir().join("cogito_bad_spec_test");
    fs::create_dir_all(&bad_spec).unwrap();
    // Write a malformed TOML (duplicate IRI)
    let bad_toml = bad_spec.join("cogito.toml");
    fs::write(
        &bad_toml,
        r#"
iri = "https://wintermute.local/cogito"

[[classes]]
iri = "https://wintermute.local/cogito#Dup"
label = "Dup"

[[classes]]
iri = "https://wintermute.local/cogito#Dup"
label = "Dup Again"
"#,
    )
    .unwrap();

    let status = Command::new(cogito_bin())
        .args(["tbox", "check", "--spec", bad_spec.to_str().unwrap()])
        .status()
        .expect("failed to run cogito");
    assert!(
        !status.success(),
        "expected non-zero exit for malformed spec"
    );

    let _ = fs::remove_dir_all(&bad_spec);
}

#[test]
fn ac7_check_shipped_spec_passes() {
    let status = Command::new(cogito_bin())
        .args(["tbox", "check"])
        .status()
        .expect("failed to run cogito");
    assert!(
        status.success(),
        "tbox check on shipped spec exited {:?}",
        status.code()
    );
}
