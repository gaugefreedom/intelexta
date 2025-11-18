/**
 * Generate a test CAR-Lite bundle with chatgpt-summarizer model
 */

import { generateProofBundle } from './dist/provenance.js';
import { writeFileSync } from 'fs';
import JSZip from 'jszip';

async function main() {
  console.log('Generating CAR-Lite bundle with chatgpt-summarizer...');

  const { bundle, isSigned } = await generateProofBundle(
    {
      url: 'https://example.com/test-article',
      content: 'This is a test article about artificial intelligence and machine learning. It covers various topics including neural networks, deep learning, and natural language processing.'
    },
    'A brief summary about AI and ML, covering neural networks and NLP.',
    'chatgpt-summarizer'
  );

  // Parse and verify the car.json
  const carData = JSON.parse(bundle['car.json']);

  console.log('\n✓ CAR bundle generated');
  console.log('  CAR ID:', carData.id);
  console.log('  Run ID:', carData.run_id);
  console.log('  Model (run):', carData.run.model);
  console.log('  Model (step):', carData.run.steps[0].model);
  console.log('  Signed:', isSigned);

  // Verify the model names
  if (carData.run.model === 'workflow:chatgpt-summarizer') {
    console.log('\n✓ run.model correctly set to "workflow:chatgpt-summarizer"');
  } else {
    console.error('\n✗ run.model is incorrect:', carData.run.model);
    process.exit(1);
  }

  if (carData.run.steps[0].model === 'chatgpt-summarizer') {
    console.log('✓ steps[0].model correctly set to "chatgpt-summarizer"');
  } else {
    console.error('✗ steps[0].model is incorrect:', carData.run.steps[0].model);
    process.exit(1);
  }

  // Create ZIP bundle
  const zip = new JSZip();

  for (const [path, content] of Object.entries(bundle)) {
    zip.file(path, content);
  }

  const zipBuffer = await zip.generateAsync({ type: 'nodebuffer' });
  const outputPath = '/tmp/mcp-sample.car.zip';
  writeFileSync(outputPath, zipBuffer);

  console.log('\n✓ CAR bundle saved to:', outputPath);
  console.log('\nYou can now test this in the web-verifier!');
}

main().catch(console.error);
