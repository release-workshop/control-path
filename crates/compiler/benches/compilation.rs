//! Performance benchmarks for Control Path compiler
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.
//!
//! These benchmarks measure compilation performance for different flag counts and scenarios.
//! Current benchmarks focus on basic compilation performance. Future enhancements could include:
//! - Complex expressions (nested conditions, multiple operators)
//! - Segments
//! - More variation in rule complexity

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use controlpath_compiler::{parse_definitions, parse_deployment, compile, serialize};

/// Generate flag definitions YAML for testing
fn generate_flag_definitions(count: usize) -> String {
    let mut flags = Vec::new();
    
    for i in 0..count {
        if i % 3 == 0 {
            // Multivariate flag every 3rd flag
            flags.push(format!(
                r#"  - name: flag_{}
    type: multivariate
    defaultValue: variation_a
    variations:
      - name: VARIATION_A
        value: variation_a
      - name: VARIATION_B
        value: variation_b
      - name: VARIATION_C
        value: variation_c"#,
                i
            ));
        } else {
            // Boolean flag
            flags.push(format!(
                r#"  - name: flag_{}
    type: boolean
    defaultValue: OFF"#,
                i
            ));
        }
    }
    
    format!("flags:\n{}", flags.join("\n"))
}

/// Generate deployment YAML for testing
fn generate_deployment(flag_count: usize) -> String {
    let mut rules = Vec::new();
    
    for i in 0..flag_count {
        let rule = if i % 5 == 0 {
            // Simple serve rule (20% of flags)
            format!("  flag_{}:\n    rules:\n      - serve: ON", i)
        } else if i % 5 == 1 {
            // Serve rule with expression (20% of flags)
            format!(
                r#"  flag_{}:
    rules:
      - serve: ON
        when: 'user.role == "admin"'"#,
                i
            )
        } else if i % 5 == 2 && i % 3 == 0 {
            // Variations rule for multivariate flags (20% of flags, subset of multivariate)
            format!(
                r#"  flag_{}:
    rules:
      - variations:
          - variation: VARIATION_A
            weight: 50
          - variation: VARIATION_B
            weight: 30
          - variation: VARIATION_C
            weight: 20"#,
                i
            )
        } else if i % 5 == 3 {
            // Rollout rule (20% of flags)
            let variation = if i % 3 == 0 { "VARIATION_A" } else { "ON" };
            format!(
                r#"  flag_{}:
    rules:
      - rollout:
          variation: {}
          percentage: 25"#,
                i, variation
            )
        } else {
            // Default rule only (20% of flags)
            format!("  flag_{}: {{}}", i)
        };
        rules.push(rule);
    }
    
    format!(
        "environment: production\nrules:\n{}",
        rules.join("\n")
    )
}

/// Benchmark compilation time for different flag counts
fn benchmark_compilation(c: &mut Criterion) {
    let flag_counts = vec![10, 50, 100, 250, 500];
    
    let mut group = c.benchmark_group("compilation");
    group.sample_size(20); // More samples for better accuracy
    
    for count in flag_counts {
        let definitions_yaml = generate_flag_definitions(count);
        let deployment_yaml = generate_deployment(count);
        
        group.bench_with_input(
            BenchmarkId::new("compile", count),
            &(definitions_yaml, deployment_yaml),
            |b, (defs, depl)| {
                b.iter(|| {
                    let definitions = parse_definitions(black_box(defs))
                        .expect("Parsing definitions should succeed in benchmarks");
                    let deployment = parse_deployment(black_box(depl))
                        .expect("Parsing deployment should succeed in benchmarks");
                    let artifact = compile(black_box(&deployment), black_box(&definitions))
                        .expect("Compilation should succeed in benchmarks");
                    black_box(artifact)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark full compilation pipeline (parse + compile + serialize)
fn benchmark_full_pipeline(c: &mut Criterion) {
    let flag_counts = vec![10, 50, 100, 250, 500];
    
    let mut group = c.benchmark_group("full_pipeline");
    group.sample_size(20);
    
    for count in flag_counts {
        let definitions_yaml = generate_flag_definitions(count);
        let deployment_yaml = generate_deployment(count);
        
        group.bench_with_input(
            BenchmarkId::new("parse_compile_serialize", count),
            &(definitions_yaml, deployment_yaml),
            |b, (defs, depl)| {
                b.iter(|| {
                    let definitions = parse_definitions(black_box(defs))
                        .expect("Parsing definitions should succeed in benchmarks");
                    let deployment = parse_deployment(black_box(depl))
                        .expect("Parsing deployment should succeed in benchmarks");
                    let artifact = compile(black_box(&deployment), black_box(&definitions))
                        .expect("Compilation should succeed in benchmarks");
                    let bytes = serialize(black_box(&artifact))
                        .expect("Serialization should succeed in benchmarks");
                    black_box(bytes)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark artifact size for different flag counts
/// 
/// Note: This benchmark measures size, but uses iter() for consistency with other benchmarks.
/// The timing is not relevant, only the size value.
fn benchmark_artifact_size(c: &mut Criterion) {
    let flag_counts = vec![10, 50, 100, 250, 500, 1000];
    
    let mut group = c.benchmark_group("artifact_size");
    
    for count in flag_counts {
        let definitions_yaml = generate_flag_definitions(count);
        let deployment_yaml = generate_deployment(count);
        
        group.bench_function(BenchmarkId::new("size_bytes", count), |b| {
            b.iter(|| {
                let definitions = parse_definitions(black_box(&definitions_yaml))
                    .expect("Parsing definitions should succeed in benchmarks");
                let deployment = parse_deployment(black_box(&deployment_yaml))
                    .expect("Parsing deployment should succeed in benchmarks");
                let artifact = compile(black_box(&deployment), black_box(&definitions))
                    .expect("Compilation should succeed in benchmarks");
                let bytes = serialize(black_box(&artifact))
                    .expect("Serialization should succeed in benchmarks");
                black_box(bytes.len())
            });
        });
    }
    
    group.finish();
}

/// Benchmark parsing performance
fn benchmark_parsing(c: &mut Criterion) {
    let flag_counts = vec![10, 50, 100, 250, 500];
    
    let mut group = c.benchmark_group("parsing");
    group.sample_size(20);
    
    for count in flag_counts {
        let definitions_yaml = generate_flag_definitions(count);
        let deployment_yaml = generate_deployment(count);
        
        group.bench_with_input(
            BenchmarkId::new("parse_definitions", count),
            &definitions_yaml,
            |b, defs| {
                b.iter(|| {
                    let definitions = parse_definitions(black_box(defs))
                        .expect("Parsing definitions should succeed in benchmarks");
                    black_box(definitions)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("parse_deployment", count),
            &deployment_yaml,
            |b, depl| {
                b.iter(|| {
                    let deployment = parse_deployment(black_box(depl))
                        .expect("Parsing deployment should succeed in benchmarks");
                    black_box(deployment)
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_compilation,
    benchmark_full_pipeline,
    benchmark_artifact_size,
    benchmark_parsing
);
criterion_main!(benches);

