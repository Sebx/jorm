#!/usr/bin/env python3
"""
Jorm Test Runner - Comprehensive test execution script
Supports running tests by phase, category, or specific features
"""
import sys
import subprocess
import argparse
from pathlib import Path


def run_command(cmd, description=""):
    """Run a command and return the result"""
    if description:
        print(f"\n🔄 {description}")
    print(f"Running: {' '.join(cmd)}")
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode == 0:
        print(f"✅ Success")
        if result.stdout:
            print(result.stdout)
    else:
        print(f"❌ Failed (exit code: {result.returncode})")
        if result.stderr:
            print("STDERR:", result.stderr)
        if result.stdout:
            print("STDOUT:", result.stdout)
    
    return result.returncode == 0


def run_tests_by_phase(phase):
    """Run tests for a specific development phase"""
    phase_markers = {
        "1": "phase1",
        "2": "phase2", 
        "3": "phase3",
        "4": "phase4",
        "5": "phase5"
    }
    
    if phase not in phase_markers:
        print(f"❌ Invalid phase: {phase}. Valid phases: {', '.join(phase_markers.keys())}")
        return False
    
    marker = phase_markers[phase]
    cmd = ["python", "-m", "pytest", "-m", marker, "-v"]
    return run_command(cmd, f"Running Phase {phase} tests")


def run_tests_by_category(category):
    """Run tests by category (unit, integration, e2e, etc.)"""
    valid_categories = ["unit", "integration", "e2e", "ai", "security", "performance", "slow"]
    
    if category not in valid_categories:
        print(f"❌ Invalid category: {category}. Valid categories: {', '.join(valid_categories)}")
        return False
    
    cmd = ["python", "-m", "pytest", "-m", category, "-v"]
    return run_command(cmd, f"Running {category} tests")


def run_all_tests():
    """Run all tests with coverage"""
    cmd = [
        "python", "-m", "pytest",
        "--cov=jorm",
        "--cov-report=html",
        "--cov-report=term-missing",
        "--cov-fail-under=80",
        "-v"
    ]
    return run_command(cmd, "Running all tests with coverage")


def run_fast_tests():
    """Run only fast tests (exclude slow and e2e)"""
    cmd = [
        "python", "-m", "pytest",
        "-m", "not slow and not e2e",
        "-v"
    ]
    return run_command(cmd, "Running fast tests only")


def run_tdd_tests():
    """Run tests in TDD mode (watch for changes)"""
    try:
        cmd = [
            "python", "-m", "pytest",
            "--tb=short",
            "-v",
            "--color=yes"
        ]
        print("🔄 Running tests in TDD mode (Ctrl+C to stop)")
        subprocess.run(cmd)
    except KeyboardInterrupt:
        print("\n⏹️  TDD mode stopped")


def run_specific_file(test_file):
    """Run tests from a specific file"""
    test_path = Path(f"tests/{test_file}")
    if not test_path.exists():
        test_path = Path(test_file)
    
    if not test_path.exists():
        print(f"❌ Test file not found: {test_file}")
        return False
    
    cmd = ["python", "-m", "pytest", str(test_path), "-v"]
    return run_command(cmd, f"Running tests from {test_file}")


def lint_code():
    """Run code linting"""
    success = True
    
    # Black formatting check
    if not run_command(["black", "--check", "jorm", "tests"], "Checking code formatting with Black"):
        success = False
    
    # isort import sorting check
    if not run_command(["isort", "--check-only", "jorm", "tests"], "Checking import sorting with isort"):
        success = False
    
    # Flake8 linting
    if not run_command(["flake8", "jorm", "tests"], "Running flake8 linting"):
        success = False
    
    # MyPy type checking
    if not run_command(["mypy", "jorm"], "Running MyPy type checking"):
        success = False
    
    return success


def security_check():
    """Run security checks"""
    success = True
    
    # Bandit security linting
    if not run_command(["bandit", "-r", "jorm"], "Running Bandit security check"):
        success = False
    
    # Safety dependency check
    if not run_command(["safety", "check"], "Checking dependencies with Safety"):
        success = False
    
    return success


def install_test_deps():
    """Install test dependencies"""
    cmd = ["pip", "install", "-r", "requirements-test.txt"]
    return run_command(cmd, "Installing test dependencies")


def main():
    """Main test runner function"""
    parser = argparse.ArgumentParser(description="Jorm Test Runner")
    parser.add_argument("--phase", "-p", choices=["1", "2", "3", "4", "5"], 
                       help="Run tests for specific development phase")
    parser.add_argument("--category", "-c", 
                       choices=["unit", "integration", "e2e", "ai", "security", "performance", "slow"],
                       help="Run tests by category")
    parser.add_argument("--file", "-f", help="Run tests from specific file")
    parser.add_argument("--all", "-a", action="store_true", help="Run all tests with coverage")
    parser.add_argument("--fast", action="store_true", help="Run only fast tests")
    parser.add_argument("--tdd", action="store_true", help="Run in TDD mode")
    parser.add_argument("--lint", action="store_true", help="Run code linting")
    parser.add_argument("--security", action="store_true", help="Run security checks")
    parser.add_argument("--install-deps", action="store_true", help="Install test dependencies")
    parser.add_argument("--ci", action="store_true", help="Run full CI pipeline")
    
    args = parser.parse_args()
    
    # Install dependencies if requested
    if args.install_deps:
        if not install_test_deps():
            sys.exit(1)
        return
    
    # Run full CI pipeline
    if args.ci:
        print("🚀 Running full CI pipeline")
        success = True
        
        if not lint_code():
            success = False
        
        if not security_check():
            success = False
        
        if not run_all_tests():
            success = False
        
        if success:
            print("✅ CI pipeline completed successfully")
        else:
            print("❌ CI pipeline failed")
            sys.exit(1)
        return
    
    # Run specific test categories
    if args.phase:
        success = run_tests_by_phase(args.phase)
    elif args.category:
        success = run_tests_by_category(args.category)
    elif args.file:
        success = run_specific_file(args.file)
    elif args.all:
        success = run_all_tests()
    elif args.fast:
        success = run_fast_tests()
    elif args.tdd:
        run_tdd_tests()
        return
    elif args.lint:
        success = lint_code()
    elif args.security:
        success = security_check()
    else:
        # Default: run fast tests
        success = run_fast_tests()
    
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()