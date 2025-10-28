import { render, screen } from '@testing-library/react';
import WorkflowViewer from '../WorkflowViewer';
import type { VerificationReport } from '../../types/verifier';

const baseReport: VerificationReport = {
  status: 'verified',
  car_id: 'car:123',
  run_id: 'run-1',
  created_at: '2024-05-05T12:00:00Z',
  signer: { public_key: 'did:key:sample' },
  model: { name: 'gpt-test', version: '1.0', kind: 'text' },
  summary: {
    checkpoints_verified: 3,
    checkpoints_total: 3,
    provenance_verified: 2,
    provenance_total: 2,
    attachments_verified: 2,
    attachments_total: 2,
    hash_chain_valid: true,
    signatures_valid: true,
    content_integrity_valid: true
  },
  workflow: {
    steps: [
      {
        key: 'prompt',
        label: 'Prompt review',
        status: 'passed',
        details: [
          { label: 'Prompt', value: 'Explain the color of the sky.' },
          { label: 'Attachment files', value: '2/2 verified' }
        ]
      },
      {
        key: 'attachments',
        label: 'Attachment integrity',
        status: 'failed',
        error: 'Attachment checksum mismatch',
        details: []
      }
    ]
  },
  error: undefined
};

describe('WorkflowViewer', () => {
  it('renders workflow steps with content and attachment information', () => {
    render(<WorkflowViewer report={baseReport} />);

    expect(screen.getByRole('heading', { name: 'Verification Timeline' })).toBeInTheDocument();
    expect(screen.getByText('Step 1: Prompt review')).toBeInTheDocument();
    expect(screen.getByText('Explain the color of the sky.')).toBeInTheDocument();
    expect(screen.getByText('2/2 verified')).toBeInTheDocument();
  });

  it('shows fallback messaging when a step is missing attachments and surfaces errors', () => {
    render(<WorkflowViewer report={baseReport} />);

    expect(screen.getByText('Attachment checksum mismatch')).toBeInTheDocument();
    expect(screen.getAllByText('No attachments were referenced for this step.')).toHaveLength(1);
  });

  it('renders an empty state when no steps are present', () => {
    const emptyReport: VerificationReport = { ...baseReport, workflow: { steps: [] } };
    render(<WorkflowViewer report={emptyReport} />);

    expect(screen.getByText('No workflow steps were returned by the verifier.')).toBeInTheDocument();
  });
});
