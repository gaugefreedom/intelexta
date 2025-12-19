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
    expect(carData.budgets).toBeDefined();
    expect(carData.provenance).toHaveLength(4);
    expect(carData.checkpoints).toHaveLength(1);
    expect(carData.sgrade).toEqual({
      score: 0,
      components: {
        provenance: 0,
        energy: 0,
        replay: 0,
        consent: 0,
        incidents: 0
      }
    });

    const attachmentKeys = Object.keys(bundle);
    const metadataAttachment = attachmentKeys.find((k) => k.endsWith('.json') && k.startsWith('attachments/'));
    expect(metadataAttachment).toBeDefined();
    // Summary attachment + metadata only when includeSource=false
    const attachmentCount = attachmentKeys.filter((key) => key.startsWith('attachments/')).length;
    expect(attachmentCount).toBe(2);
  });

  it('generates signed car.json when secret key is provided', async () => {
    // Use a test keypair (this is just for testing)
    const testSecretKey = 'xOGXc6uj9joBKzixb22wLAPJPd4jDt5I16iWfMVkRu000sfAatteVwuvS7dCuivMyWaIRgvVxu7xHDq8P5U1cA==';

    const { bundle, isSigned } = await generateProofBundle(
      { url: 'inline://test', content: 'Example content for testing.' },
      'Test summary',
      'test-model',
      testSecretKey,
      { includeSource: true }
    );

    expect(isSigned).toBe(true);

    const carData = JSON.parse(bundle['car.json']);

    // Check signed fields
    expect(carData.signer_public_key).toBeDefined();
    expect(carData.signer_public_key.length).toBeGreaterThan(0);

    // Check dual signatures (body + checkpoint)
    expect(carData.signatures).toHaveLength(2);
    expect(carData.signatures[0]).toMatch(/^ed25519-body:.+/);
    expect(carData.signatures[1]).toMatch(/^ed25519-checkpoint:.+/);

    // Verify deterministic ID
    expect(carData.id).toMatch(/^car:[0-9a-f]{64}$/);

    const attachmentKeys = Object.keys(bundle);
    const metadataAttachment = attachmentKeys.find((k) => k.endsWith('.json') && k.startsWith('attachments/'));
    expect(metadataAttachment).toBeDefined();
    const sourceAttachment = attachmentKeys.find(
      (key) => {
        if (!key.startsWith('attachments/') || !key.endsWith('.txt')) {
          return false;
        }
        const value = bundle[key as `attachments/${string}`];
        return value.includes('Example content for testing.');
      }
    );
    expect(sourceAttachment).toBeDefined();
  });
});
