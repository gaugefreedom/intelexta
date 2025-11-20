import type { FileError } from 'react-dropzone';

export const ACCEPTED_PROOF_EXTENSIONS = ['.car.json', '.car.zip'] as const;

export type ProofFileExtension = (typeof ACCEPTED_PROOF_EXTENSIONS)[number];
export type ProofFileKind = 'json' | 'car';

export const PROOF_FILE_ACCEPT_MESSAGE =
  'Upload a .car.json transcript or a .car.zip archive exported from Intelexta.';

export function normalizeFileName(fileName: string): string {
  return fileName.trim().toLowerCase();
}

/**
 * Strip browser download suffixes like (1), (2), etc. from filename
 * Examples:
 *   "file.car.zip" -> "file.car.zip"
 *   "file.car(1).zip" -> "file.car.zip"
 *   "file.car (2).zip" -> "file.car.zip"
 *   "file(1).car.json" -> "file.car.json"
 */
function stripBrowserSuffix(fileName: string): string {
  // Remove " (N)" or "(N)" pattern before the last extension
  // Match pattern: optional space + (digits) before the extension
  return fileName.replace(/\s*\(\d+\)(?=\.[^.]+$)/, '');
}

export function getProofFileExtension(fileName: string): ProofFileExtension | null {
  let normalized = normalizeFileName(fileName);

  // Strip browser download suffixes (1), (2), etc.
  normalized = stripBrowserSuffix(normalized);

  // Accept exact "car.json" filename (from extracted bundles)
  if (normalized === 'car.json') {
    return '.car.json';
  }

  // Accept "car_*.json" pattern (validator outputs)
  if (normalized.startsWith('car_') && normalized.endsWith('.json')) {
    return '.car.json';
  }

  return ACCEPTED_PROOF_EXTENSIONS.find((extension) => normalized.endsWith(extension)) ?? null;
}

export function getProofFileKind(fileName: string): ProofFileKind | null {
  const extension = getProofFileExtension(fileName);
  if (!extension) return null;
  return extension === '.car.json' ? 'json' : 'car';
}

export function validateProofFileName(fileName: string):
  | { valid: true; kind: ProofFileKind }
  | { valid: false; message: string } {
  const kind = getProofFileKind(fileName);
  if (!kind) {
    return { valid: false, message: PROOF_FILE_ACCEPT_MESSAGE };
  }
  return { valid: true, kind };
}

export function buildProofDropzoneAccept(): Record<string, string[]> {
  return {
    'application/json': ['.car.json', '.json'],
    'application/zip': ['.car.zip'],
    'application/x-zip-compressed': ['.car.zip'],
    'application/octet-stream': ['.car.zip']
  };
}

export type ProofFileValidationError = FileError & { code: 'file-invalid-type' };
export type FileLike = Pick<File, 'name'> | { name: string };

export function proofFileValidator(file: FileLike): ProofFileValidationError | null {
  // Safety check for undefined filename
  if (!file || !file.name) {
    return {
      code: 'file-invalid-type',
      message: PROOF_FILE_ACCEPT_MESSAGE
    };
  }

  const validation = validateProofFileName(file.name);
  if (!validation.valid) {
    return {
      code: 'file-invalid-type',
      message: validation.message
    };
  }
  return null;
}
