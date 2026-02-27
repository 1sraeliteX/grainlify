# Deployment Script Failure Handling Tests

This document describes the comprehensive test suite for validating failure handling in Grainlify deployment scripts.

## Overview

The test suite ensures that all deployment scripts:
- Exit with non-zero status codes on failure
- Display helpful error messages
- Validate inputs properly
- Handle missing dependencies gracefully
- Manage configuration file issues appropriately

## Test Scripts

### 1. `test_all_script_failures.sh` (Comprehensive Test Suite)

The main test harness that covers all deployment scripts with extensive failure scenarios.

**Usage:**
```bash
./scripts/test_all_script_failures.sh [options]
```

**Options:**
- `-v, --verbose` - Show detailed test output
- `-q, --quiet` - Only show failures
- `-h, --help` - Show help message

### 2. `test_deploy_failures.sh` (Deploy Script Tests)

Focused tests for the `deploy.sh` script.

### 3. `test_upgrade_failures.sh` (Upgrade Script Tests)

Focused tests for the `upgrade.sh` script.

## Tested Scripts

### deploy.sh
**Purpose:** Deploys Soroban smart contracts to specified networks

**Tested Failure Scenarios:**

#### Input Validation
- ✅ Missing WASM file argument
- ✅ Non-existent WASM file path
- ✅ Invalid WASM file format (wrong magic header)
- ✅ Empty WASM file
- ✅ Invalid network name
- ✅ Multiple WASM arguments (should fail)
- ✅ Unknown command line options

#### Identity & Authentication
- ✅ Invalid/missing deployer identity
- ✅ Identity verification failures

#### Dependencies
- ✅ Missing Stellar/Soroban CLI tools
- ✅ CLI tool installation verification

#### Configuration
- ✅ Missing configuration files (should warn but continue)
- ✅ Invalid configuration file format
- ✅ Environment variable validation

#### Functionality
- ✅ Help command works without dependencies
- ✅ Dry run mode with valid inputs
- ✅ Simulated deployment failures

### upgrade.sh
**Purpose:** Upgrades existing contracts to new WASM versions

**Tested Failure Scenarios:**

#### Input Validation
- ✅ Missing contract ID
- ✅ Invalid contract ID format
- ✅ Missing WASM file argument
- ✅ Non-existent WASM file
- ✅ Invalid WASM file format
- ✅ Empty WASM file
- ✅ Invalid network name
- ✅ Unknown command line options

#### Identity & Authentication
- ✅ Invalid/missing source identity
- ✅ Identity verification failures

#### Configuration
- ✅ Missing configuration files (should warn but continue)
- ✅ Environment variable validation

#### Functionality
- ✅ Help command works without dependencies
- ✅ Dry run mode with valid inputs

### rollback.sh
**Purpose:** Rolls back contracts to previous WASM versions

**Tested Failure Scenarios:**

#### Input Validation
- ✅ Missing contract ID
- ✅ Missing previous WASM hash
- ✅ Invalid contract ID format
- ✅ Invalid WASM hash format
- ✅ Invalid network name
- ✅ Unknown command line options

#### Identity & Authentication
- ✅ Invalid/missing source identity
- ✅ Identity verification failures

### verify-deployment.sh
**Purpose:** Verifies contract health and responsiveness

**Tested Failure Scenarios:**

#### Input Validation
- ✅ Missing contract ID
- ✅ Invalid contract ID format
- ✅ Invalid network name
- ✅ Check admin without expected admin address
- ✅ Unknown command line options

## Test Categories

### 1. Input Validation Tests
Ensure scripts properly validate:
- Required arguments
- File existence and format
- Contract ID formats
- Network names
- Command line options

### 2. Configuration Tests
Validate handling of:
- Missing configuration files
- Invalid configuration syntax
- Environment variable defaults
- Configuration precedence (CLI > config > env > defaults)

### 3. Dependency Tests
Check behavior when:
- CLI tools are missing
- Network connectivity fails
- Identity verification fails
- Required commands are unavailable

### 4. Error Message Tests
Verify that error messages are:
- Descriptive and helpful
- Include relevant context
- Guide users toward solutions
- Are consistently formatted

### 5. Exit Code Tests
Ensure scripts exit with:
- Non-zero codes on failure
- Zero codes on success
- Different codes for different error types (where applicable)

## Mock Framework

The test suite uses a comprehensive mock framework to simulate:

### CLI Tool Mocking
- **Stellar CLI**: Mocks `stellar keys address` and `stellar contract` commands
- **Soroban CLI**: Basic mock for backward compatibility
- **Failure Simulation**: Environment variables to trigger specific failures

### Test Data
- **Valid WASM**: Minimal WASM file with correct magic header
- **Invalid WASM**: Files with wrong headers or empty content
- **Configuration Files**: Both valid and malformed config examples

### Identity Mocking
- Simulates existing and non-existent identities
- Tests identity verification flows
- Validates authentication error handling

## Running Tests

### Local Development
```bash
# Run comprehensive test suite
./scripts/test_all_script_failures.sh

# Run with verbose output
./scripts/test_all_script_failures.sh -v

# Run individual script tests
./scripts/test_deploy_failures.sh
./scripts/test_upgrade_failures.sh
```

### CI/CD Pipeline
Tests are automatically run in GitHub Actions on:
- Pull requests affecting contracts or scripts
- Pushes to main/develop branches

### Test Output Format
```
✔ PASS: Test description
✘ FAIL: Test description (expected: X, got: Y)
ℹ INFO: Additional information
```

## Adding New Tests

When adding new failure scenarios:

1. **Update Test Scripts**: Add test cases to appropriate test files
2. **Mock New Scenarios**: Extend mock framework if needed
3. **Update Documentation**: Document new test scenarios here
4. **Verify CI**: Ensure tests pass in CI environment

### Test Template
```bash
# Test description
run_expect_fail "Descriptive test name" "Expected error message" \
    script_name arguments
```

## Coverage Goals

The test suite aims for comprehensive coverage of:
- ✅ All command line options and arguments
- ✅ All error conditions and edge cases
- ✅ All configuration scenarios
- ✅ All dependency failure modes
- ✅ All user-facing error messages

## Continuous Improvement

This test suite should be updated when:
- New scripts are added
- Existing scripts gain new features
- New failure modes are discovered
- Error messages are improved
- Configuration options change

## Troubleshooting

### Common Issues

1. **Mock Conflicts**: Ensure test isolation and proper cleanup
2. **Path Issues**: Use absolute paths for test files
3. **Permission Errors**: Ensure scripts are executable
4. **Environment State**: Clean environment variables between tests

### Debug Mode
Run with verbose output to see detailed test execution:
```bash
./scripts/test_all_script_failures.sh -v
```

## Future Enhancements

Potential improvements to consider:
- Integration tests with real Stellar network
- Performance testing for large deployments
- Parallel test execution
- Test coverage reporting
- Automated test case generation from script help text
