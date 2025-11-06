import { describe, expect, it } from 'vitest';

const { internal } = await import('./index.js');

const limit = 10;

describe('readResponseBodyWithLimit', () => {
  it('returns body when Content-Length is within the limit', async () => {
    const response = new Response('hello', {
      headers: {
        'content-length': '5'
      }
    });

    const content = await internal.readResponseBodyWithLimit(response, limit);

    expect(content).toBe('hello');
  });

  it('rejects when Content-Length exceeds the limit', async () => {
    const response = new Response('toolarge', {
      headers: {
        'content-length': '20'
      }
    });

    await expect(
      internal.readResponseBodyWithLimit(response, limit)
    ).rejects.toThrowError('Remote file is too large. Maximum allowed size is 10 bytes.');
  });

  it('streams the body when Content-Length is missing', async () => {
    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode('abc'));
        controller.enqueue(encoder.encode('def'));
        controller.close();
      }
    });

    const response = new Response(stream);

    const content = await internal.readResponseBodyWithLimit(response, limit);

    expect(content).toBe('abcdef');
  });

  it('aborts streaming downloads that exceed the limit without Content-Length', async () => {
    const encoder = new TextEncoder();
    const chunks = [encoder.encode('12345'), encoder.encode('67890')];
    let cancelledWith: unknown;

    const fakeResponse = {
      headers: new Headers(),
      body: {
        getReader() {
          return {
            async read() {
              const chunk = chunks.shift();
              if (!chunk) {
                return { value: undefined, done: true };
              }
              return { value: chunk, done: false };
            },
            releaseLock() {
              // no-op for tests
            }
          };
        },
        async cancel(reason: unknown) {
          cancelledWith = reason;
        }
      }
    } as unknown as Response;

    await expect(
      internal.readResponseBodyWithLimit(fakeResponse, limit - 1)
    ).rejects.toThrowError('Remote file is too large. Maximum allowed size is 9 bytes.');

    expect(cancelledWith).toBe('Exceeded size limit');
  });
});
