import { describe, it, expect } from 'vitest';

import { generateProofBundle } from './provenance.js';

describe('generateProofBundle', () => {
  it('generates CAR-Lite compliant car.json when no secret key is provided', async () => {
    const { bundle, isSigned } = await generateProofBundle(
      { url: 'inline://test', content: 'Example content for testing.' },
      'Test summary',
      'test-model'
    );

    expect(isSigned).toBe(false);

    const carData = JSON.parse(bundle['car.json']);

    // Check required v0.2 fields
    expect(carData.id).toMatch(/^car:[0-9a-f]{64}$/);
    expect(carData.run_id).toBeDefined();
    expect(carData.created_at).toBeDefined();
    expect(carData.signer_public_key).toBe('');
    expect(carData.signatures).toEqual(['unsigned:']);

    // Check run object
    expect(carData.run.kind).toBe('concordant');
    expect(carData.run.name).toBe('verifiable summary');
    expect(carData.run.steps).toHaveLength(1);

    // Check CAR-Lite required fields
    expect(carData.proof.match_kind).toBe('process');
    expect(carData.budgets).toEqual({ usd: 0, tokens: 0, nature_cost: 0 });
    expect(carData.provenance).toHaveLength(3);
    expect(carData.checkpoints).toHaveLength(1);
    expect(carData.sgrade.score).toBe(85);
  });

  it('generates signed car.json when secret key is provided', async () => {
    // Use a test keypair (this is just for testing)
    const testSecretKey = 'xOGXc6uj9joBKzixb22wLAPJPd4jDt5I16iWfMVkRu000sfAatteVwuvS7dCuivMyWaIRgvVxu7xHDq8P5U1cA==';

    const { bundle, isSigned } = await generateProofBundle(
      { url: 'inline://test', content: 'Example content for testing.' },
      'Test summary',
      'test-model',
      testSecretKey
    );

    expect(isSigned).toBe(true);

    const carData = JSON.parse(bundle['car.json']);

    // Check signed fields
    expect(carData.signer_public_key).toBeDefined();
    expect(carData.signer_public_key.length).toBeGreaterThan(0);
    expect(carData.signatures).toHaveLength(1);
    expect(carData.signatures[0]).toMatch(/^ed25519:.+/);

    // Verify deterministic ID
    expect(carData.id).toMatch(/^car:[0-9a-f]{64}$/);
  });
});
