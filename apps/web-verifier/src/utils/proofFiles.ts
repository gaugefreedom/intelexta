import type { FileError } from 'react-dropzone';

export const ACCEPTED_PROOF_EXTENSIONS = ['.car.json', '.car.zip'] as const;

export type ProofFileExtension = (typeof ACCEPTED_PROOF_EXTENSIONS)[number];
export type ProofFileKind = 'json' | 'car';

export const PROOF_FILE_ACCEPT_MESSAGE =
  'Upload a .car.json transcript or a .car.zip archive exported from IntelexTA.';

export function normalizeFileName(fileName: string): string {
  return fileName.trim().toLowerCase();
}

export function getProofFileExtension(fileName: string): ProofFileExtension | null {
  const normalized = normalizeFileName(fileName);
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
    'application/json': ['.car.json'],
    'application/zip': ['.car.zip'],
    'application/x-zip-compressed': ['.car.zip'],
    'application/octet-stream': ['.car.zip']
  };
}

export type ProofFileValidationError = FileError & { code: 'file-invalid-type' };
export type FileLike = Pick<File, 'name'> | { name: string };

export function proofFileValidator(file: FileLike): ProofFileValidationError | null {
  const validation = validateProofFileName(file.name);
  if (!validation.valid) {
    return {
      code: 'file-invalid-type',
      message: validation.message
    };
  }
  return null;
}
