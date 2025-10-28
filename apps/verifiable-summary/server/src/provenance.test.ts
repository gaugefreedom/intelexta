import { describe, it, expect } from 'vitest';

import { generateProofBundle } from './provenance.js';

describe('generateProofBundle', () => {
  it('marks bundles as unsigned when no secret key is provided', async () => {
    const { bundle, isSigned } = await generateProofBundle(
      { url: 'inline://test', content: 'Example content for testing.' },
      'Test summary',
      'test-model'
    );

    expect(isSigned).toBe(false);

    const receipt = JSON.parse(bundle['receipts/ed25519.json']);
    expect(receipt.algorithm).toBe('none');
    expect(receipt.note).toContain('Unsigned bundle');
  });
});
