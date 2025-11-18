#!/usr/bin/env node
/**
 * CAR Schema Validator for Intelexta
 *
 * Validates CAR JSON files against the official CAR v0.3 schema.
 *
 * Usage:
 *   node validate-car.js <path-to-car.json>
 *   node validate-car.js <path-to-bundle.car.zip>
 *
 * Examples:
 *   node validate-car.js examples/desktop-car.json
 *   node validate-car.js ~/Downloads/my-bundle.car.zip
 */

const fs = require('fs');
const path = require('path');

// Determine if we need to extract from ZIP
const carPath = process.argv[2];

if (!carPath) {
  console.error('Usage: node validate-car.js <car.json | bundle.car.zip>');
  process.exit(1);
}

// Simple validation without external dependencies
function validateCAR(carJson) {
  const required = [
    'id', 'run_id', 'created_at', 'run', 'proof', 'policy_ref',
    'budgets', 'provenance', 'checkpoints', 'sgrade',
    'signer_public_key', 'signatures'
  ];

  const errors = [];

  // Check required top-level fields
  for (const field of required) {
    if (!(field in carJson)) {
      errors.push(`Missing required field: ${field}`);
    }
  }

  // Check proof.process for process match_kind
  if (carJson.proof) {
    if (carJson.proof.match_kind === 'process') {
      if (!carJson.proof.process) {
        errors.push('match_kind is "process" but proof.process is missing');
      } else if (!carJson.proof.process.sequential_checkpoints) {
        errors.push('proof.process.sequential_checkpoints is required');
      } else if (carJson.proof.process.sequential_checkpoints.length === 0) {
        errors.push('proof.process.sequential_checkpoints must have at least 1 checkpoint');
      }
    }
  }

  // Check run.steps field naming (should be camelCase)
  if (carJson.run && carJson.run.steps) {
    const step = carJson.run.steps[0];
    if (step) {
      if ('run_id' in step || 'order_index' in step || 'checkpoint_type' in step) {
        errors.push('run.steps should use camelCase (runId, orderIndex, checkpointType), not snake_case');
      }
    }
  }

  return errors;
}

try {
  let carJson;

  if (carPath.endsWith('.zip')) {
    // For ZIP files, user needs to extract manually or use a proper validator with zip support
    console.log('Note: ZIP file detected. Please extract car.json first or use the full validator.');
    console.log('  unzip -p bundle.car.zip car.json | node validate-car.js /dev/stdin');
    process.exit(1);
  } else {
    // Read JSON file
    const content = fs.readFileSync(carPath, 'utf8');
    carJson = JSON.parse(content);
  }

  // Perform basic validation
  const errors = validateCAR(carJson);

  if (errors.length === 0) {
    console.log('✓ CAR appears valid (basic check)');
    console.log(`  ID: ${carJson.id}`);
    console.log(`  Run: ${carJson.run_id}`);
    console.log(`  Match kind: ${carJson.proof?.match_kind}`);
    console.log(`  Checkpoints: ${carJson.checkpoints?.length || 0}`);
    console.log('\nFor full schema validation, install AJV:');
    console.log('  npm install ajv ajv-formats');
    console.log('  node validate-car-full.js <car.json>');
    process.exit(0);
  } else {
    console.log('✗ CAR validation errors:');
    errors.forEach((err, i) => {
      console.log(`  ${i + 1}. ${err}`);
    });
    process.exit(1);
  }

} catch (error) {
  console.error('Error:', error.message);
  process.exit(1);
}
