/// TBox subcommand implementations: build, check, stats.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use horned_owl::curie::PrefixMapping;
use horned_owl::model::*;
use horned_owl::ontology::component_mapped::RcComponentMappedOntology;

use crate::spec::{self, CogitoSpec};

const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
const OWL_TRANSITIVE: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";

/// Build the TBox OWL file.
///
/// If `ousia-forge` is on $PATH and the spec directory is provided, delegates
/// to it. Otherwise (the normal case with the built-in spec or a single-file
/// spec), emits OWL/XML directly via horned-owl.
pub fn build(spec_path: Option<&Path>, out: &Path) -> Result<()> {
    let loaded_spec = spec::load(spec_path)?;
    build_with_horned(&loaded_spec, out)?;

    // Optionally run ousia-reason check (AC5)
    if let Ok(reason_path) = which_ousia_reason() {
        eprintln!("note: running ousia-reason check on output...");
        let status = Command::new(&reason_path)
            .args(["check", &out.to_string_lossy()])
            .status()
            .with_context(|| format!("running ousia-reason at {}", reason_path))?;
        if !status.success() {
            bail!(
                "ousia-reason check failed on {}: exit {}",
                out.display(),
                status
            );
        }
        eprintln!("ousia-reason: OK");
    } else {
        eprintln!("note: ousia-reason not found on PATH; OWL 2 DL profile check skipped");
    }

    println!("Built: {}", out.display());
    Ok(())
}

/// Validate the spec without emitting output.
pub fn check(spec_path: Option<&Path>) -> Result<()> {
    spec::load(spec_path)?;
    println!("OK: spec is valid");
    Ok(())
}

/// Print class/property/axiom counts for a built OWL file.
pub fn stats(file: &Path) -> Result<()> {
    use horned_owl::io::owx::reader;
    use std::fs::File;
    use std::io::BufReader;

    let f = File::open(file).with_context(|| format!("opening {}", file.display()))?;
    let mut reader = BufReader::new(f);
    let (ont, _prefixes) = reader::read(&mut reader, Default::default())
        .map_err(|e| anyhow::anyhow!("parsing OWL/XML: {}", e))?;

    let mut class_count = 0usize;
    let mut op_count = 0usize;
    let mut axiom_count = 0usize;

    for component in ont.iter() {
        axiom_count += 1;
        match component.kind() {
            ComponentKind::DeclareClass => class_count += 1,
            ComponentKind::DeclareObjectProperty => op_count += 1,
            _ => {}
        }
    }

    println!("classes:            {}", class_count);
    println!("object_properties:  {}", op_count);
    println!("total_axioms:       {}", axiom_count);
    Ok(())
}

/// Build OWL/XML via horned-owl directly (fallback / primary for cogito).
fn build_with_horned(spec: &CogitoSpec, out: &Path) -> Result<()> {
    let b = Build::new_rc();
    let mut ont = RcComponentMappedOntology::new_rc();

    // Set ontology IRI
    let ont_iri = spec
        .iri
        .as_deref()
        .unwrap_or(spec::COGITO_ONT);
    let ont_id = OntologyID {
        iri: Some(b.iri(ont_iri)),
        viri: spec.version_iri.as_deref().map(|v| b.iri(v)),
    };
    ont.insert(ont_id);

    // owl:imports
    let imports = spec.imports.as_deref().unwrap_or(&[]);
    for import_iri in imports {
        ont.insert(Import(b.iri(import_iri.as_str())));
    }

    // Declare object properties
    for op_def in &spec.object_properties {
        let op = b.object_property(op_def.iri.as_str());
        ont.insert(DeclareObjectProperty(op.clone()));
        // rdfs:label
        ont.insert(AnnotationAssertion {
            subject: AnnotationSubject::IRI(op.0.clone()),
            ann: Annotation {
                ap: b.annotation_property(RDFS_LABEL),
                av: AnnotationValue::Literal(Literal::Simple {
                    literal: op_def.label.clone(),
                }),
            },
        });
        // owl:TransitiveProperty
        if op_def.transitive {
            ont.insert(TransitiveObjectProperty(
                ObjectPropertyExpression::ObjectProperty(op.clone()),
            ));
        }
        // inverse_of
        if let Some(inv_iri) = &op_def.inverse_of {
            let inv = b.object_property(inv_iri.as_str());
            ont.insert(InverseObjectProperties(op.clone(), inv));
        }
    }

    // Declare classes in stable order (already deterministic from TOML parse order)
    for cls in &spec.classes {
        let cls_iri = cls.iri.as_str();
        let cls_obj = b.class(cls_iri);
        ont.insert(DeclareClass(cls_obj.clone()));

        // rdfs:label
        ont.insert(AnnotationAssertion {
            subject: AnnotationSubject::IRI(cls_obj.0.clone()),
            ann: Annotation {
                ap: b.annotation_property(RDFS_LABEL),
                av: AnnotationValue::Literal(Literal::Simple {
                    literal: cls.label.clone(),
                }),
            },
        });

        // rdfs:comment (definition)
        if let Some(def) = &cls.definition {
            ont.insert(AnnotationAssertion {
                subject: AnnotationSubject::IRI(cls_obj.0.clone()),
                ann: Annotation {
                    ap: b.annotation_property(RDFS_COMMENT),
                    av: AnnotationValue::Literal(Literal::Simple {
                        literal: def.clone(),
                    }),
                },
            });
        }

        // SubClassOf (parent)
        if let Some(parent_iri) = cls.effective_parent() {
            let parent_ce = ClassExpression::Class(b.class(parent_iri));
            ont.insert(SubClassOf {
                sup: parent_ce,
                sub: ClassExpression::Class(cls_obj.clone()),
            });
        }

        // SubClassOf restrictions
        for restriction_str in &cls.subclass_of {
            let restriction_ce = parse_class_expression(&b, restriction_str)
                .with_context(|| format!("parsing subclass_of for {}", cls_iri))?;
            ont.insert(SubClassOf {
                sup: restriction_ce,
                sub: ClassExpression::Class(cls_obj.clone()),
            });
        }

        // EquivalentClasses (defined class)
        if let Some(equiv_str) = &cls.equivalent_to {
            let equiv_ce = parse_class_expression(&b, equiv_str)
                .with_context(|| format!("parsing equivalent_to for {}", cls_iri))?;
            ont.insert(EquivalentClasses(vec![
                ClassExpression::Class(cls_obj.clone()),
                equiv_ce,
            ]));
        }
    }

    // Write OWL/XML
    let mut mapping = PrefixMapping::default();
    mapping
        .add_prefix("owl", "http://www.w3.org/2002/07/owl#")
        .ok();
    mapping
        .add_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#")
        .ok();
    mapping
        .add_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
        .ok();
    mapping
        .add_prefix("xsd", "http://www.w3.org/2001/XMLSchema#")
        .ok();
    mapping.add_prefix("cogito", spec::COGITO_NS).ok();
    mapping
        .add_prefix("obo", "http://purl.obolibrary.org/obo/")
        .ok();

    let mut buf = Vec::new();
    horned_owl::io::owx::writer::write(&mut buf, &ont, Some(&mapping))
        .map_err(|e| anyhow::anyhow!("OWL/XML write error: {}", e))?;

    std::fs::write(out, &buf).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

/// Parse a class expression string of the forms:
///   "FullIRI and FullIRI"   (intersection)
///   "FullIRI some FullIRI"  (existential restriction)
///   "FullIRI"               (named class)
fn parse_class_expression(
    b: &Build<RcStr>,
    expr: &str,
) -> Result<ClassExpression<RcStr>> {
    let expr = expr.trim();

    // Try "prop some CE"
    if let Some((prop_part, rest)) = split_keyword(expr, " some ") {
        let ope = ObjectPropertyExpression::ObjectProperty(
            b.object_property(prop_part.trim()),
        );
        let bce = parse_class_expression(b, rest.trim())?;
        return Ok(ClassExpression::ObjectSomeValuesFrom {
            ope,
            bce: Box::new(bce),
        });
    }

    // Try "CE1 and CE2 and ..." (intersection at depth 0)
    if expr.contains(" and ") {
        let parts = split_conjunction(expr);
        if parts.len() > 1 {
            let ces: Result<Vec<_>> = parts
                .iter()
                .map(|p| parse_class_expression(b, p.trim()))
                .collect();
            return Ok(ClassExpression::ObjectIntersectionOf(ces?));
        }
    }

    // Strip outer parens
    let stripped = strip_parens(expr);
    if stripped != expr {
        return parse_class_expression(b, stripped);
    }

    // Named class (expect full IRI)
    Ok(ClassExpression::Class(b.class(expr)))
}

fn split_keyword<'a>(s: &'a str, kw: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0i32;
    let kw_bytes = kw.as_bytes();
    let s_bytes = s.as_bytes();
    let kw_len = kw_bytes.len();
    for i in 0..s_bytes.len() {
        match s_bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth == 0 && s_bytes[i..].starts_with(kw_bytes) {
            return Some((&s[..i], &s[i + kw_len..]));
        }
    }
    None
}

fn split_conjunction(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut last = 0;
    let bytes = s.as_bytes();
    let kw = b" and ";
    let kw_len = kw.len();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth == 0 && bytes[i..].starts_with(kw) {
            result.push(&s[last..i]);
            last = i + kw_len;
        }
    }
    result.push(&s[last..]);
    result
}

fn strip_parens(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Find ousia-reason on PATH or well-known locations.
fn which_ousia_reason() -> Result<String, ()> {
    // Check ~/.local/bin first
    let local = format!("{}/.local/bin/ousia-reason", std::env::var("HOME").unwrap_or_default());
    if std::path::Path::new(&local).exists() {
        return Ok(local);
    }
    // Check PATH
    if let Ok(output) = Command::new("which").arg("ousia-reason").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Ok(s);
            }
        }
    }
    Err(())
}
