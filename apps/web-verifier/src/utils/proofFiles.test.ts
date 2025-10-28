import { describe, expect, it } from 'vitest';
import {
  buildProofDropzoneAccept,
  getProofFileExtension,
  getProofFileKind,
  normalizeFileName,
  proofFileValidator,
  validateProofFileName
} from './proofFiles';

describe('proof file utilities', () => {
  it('normalizes file names to lowercase without whitespace', () => {
    expect(normalizeFileName('  REPORT.CAR.JSON  ')).toBe('report.car.json');
  });

  it('extracts known extensions', () => {
    expect(getProofFileExtension('report.car.json')).toBe('.car.json');
    expect(getProofFileExtension('report.car.zip')).toBe('.car.zip');
  });

  it('returns null for unsupported extensions', () => {
    expect(getProofFileExtension('report.txt')).toBeNull();
  });

  it('detects the proof file kind', () => {
    expect(getProofFileKind('report.car.json')).toBe('json');
    expect(getProofFileKind('archive.car.zip')).toBe('car');
  });

  it('validates supported file names', () => {
    expect(validateProofFileName('report.car.json')).toEqual({ valid: true, kind: 'json' });
    expect(validateProofFileName('archive.car.zip')).toEqual({ valid: true, kind: 'car' });
  });

  it('flags unsupported file names with a descriptive error', () => {
    const result = validateProofFileName('report.txt');
    expect(result.valid).toBe(false);
    if (!result.valid) {
      expect(result.message).toContain('.car.json');
      expect(result.message).toContain('.car.zip');
    }
  });

  it('provides an accept config for dropzone consumers', () => {
    const accept = buildProofDropzoneAccept();
    expect(accept['application/json']).toContain('.car.json');
    expect(accept['application/zip']).toContain('.car.zip');
  });

  it('creates dropzone validation errors for unsupported files', () => {
    expect(proofFileValidator({ name: 'archive.car.zip' })).toBeNull();
    const rejection = proofFileValidator({ name: 'archive.txt' });
    expect(rejection).not.toBeNull();
    expect(rejection?.code).toBe('file-invalid-type');
  });
});
