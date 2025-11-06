import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('node:dns/promises', () => ({
  lookup: vi.fn()
}));

const { validateRemoteFileUrl } = await import('./urlValidation.js');
const { lookup } = await import('node:dns/promises');

const mockedLookup = lookup as unknown as ReturnType<typeof vi.fn>;

describe('validateRemoteFileUrl', () => {
  beforeEach(() => {
    mockedLookup.mockReset();
  });

  it('allows http URLs that resolve to public IPs', async () => {
    mockedLookup.mockResolvedValue([
      { address: '93.184.216.34', family: 4 }
    ]);

    const result = await validateRemoteFileUrl('https://example.com/data.txt');

    expect(result.hostname).toBe('example.com');
    expect(mockedLookup).toHaveBeenCalledWith('example.com', { all: true });
  });

  it('rejects URLs that resolve to private IP addresses', async () => {
    mockedLookup.mockResolvedValue([
      { address: '10.0.0.12', family: 4 }
    ]);

    await expect(
      validateRemoteFileUrl('https://example.com/data.txt')
    ).rejects.toThrow('Resolved IP address is not allowed');
  });

  it('rejects direct private IP URLs without DNS lookup', async () => {
    await expect(
      validateRemoteFileUrl('https://192.168.1.1/secrets.txt')
    ).rejects.toThrow('IP address is not allowed');

    expect(mockedLookup).not.toHaveBeenCalled();
  });

  it('rejects URLs with unsupported protocols', async () => {
    await expect(
      validateRemoteFileUrl('ftp://example.com/data.txt')
    ).rejects.toThrow('Unsupported URL protocol');
  });
});
