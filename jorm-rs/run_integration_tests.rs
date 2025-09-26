//! Integration Test Runner
//!
//! This script replaces all PowerShell test scripts with a comprehensive Rust test runner.
//! Run with: cargo run --bin run_integration_tests

use std::path::Path;
use std::process::Command;

fn main() {
    println!("🚀 Running Jorm Integration Tests");
    println!("{}", "=".repeat(50));

    // Check if the binary exists
    if !Path::new("target/debug/jorm-rs.exe").exists() {
        println!("❌ jorm-rs binary not found. Building...");
        let build_result = Command::new("cargo")
            .args(&["build"])
            .status()
            .expect("Failed to run cargo build");

        if !build_result.success() {
            println!("❌ Build failed!");
            std::process::exit(1);
        }
        println!("✅ Build completed");
    }

    // Run all integration tests
    println!("🧪 Running integration tests...");
    let test_result = Command::new("cargo")
        .args(&["test", "--test", "comprehensive_test_suite"])
        .status()
        .expect("Failed to run tests");

    if test_result.success() {
        println!("✅ All integration tests passed!");
    } else {
        println!("❌ Some integration tests failed!");
        std::process::exit(1);
    }

    // Run CLI integration tests
    println!("🧪 Running CLI integration tests...");
    let cli_test_result = Command::new("cargo")
        .args(&["test", "--test", "cli_integration_tests"])
        .status()
        .expect("Failed to run CLI tests");

    if cli_test_result.success() {
        println!("✅ All CLI integration tests passed!");
    } else {
        println!("❌ Some CLI integration tests failed!");
        std::process::exit(1);
    }

    // Run interactive integration tests
    println!("🧪 Running interactive integration tests...");
    let interactive_test_result = Command::new("cargo")
        .args(&["test", "--test", "interactive_integration_tests"])
        .status()
        .expect("Failed to run interactive tests");

    if interactive_test_result.success() {
        println!("✅ All interactive integration tests passed!");
    } else {
        println!("❌ Some interactive integration tests failed!");
        std::process::exit(1);
    }

    // Run DAG execution integration tests
    println!("🧪 Running DAG execution integration tests...");
    let dag_test_result = Command::new("cargo")
        .args(&["test", "--test", "dag_execution_integration_tests"])
        .status()
        .expect("Failed to run DAG execution tests");

    if dag_test_result.success() {
        println!("✅ All DAG execution integration tests passed!");
    } else {
        println!("❌ Some DAG execution integration tests failed!");
        std::process::exit(1);
    }

    println!("🎉 All integration tests completed successfully!");
}
