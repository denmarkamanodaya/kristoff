import type { S3Client } from '@aws-sdk/client-s3';
import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../test/support/sdk-stub.js';
import { s3FileStore } from './v1.js';

const NOW = Date.UTC(2026, 10, 12);

const setup = (...replies: unknown[]) => {
  const { client, calls } = stubClient(...replies);
  const signed: { command: string; input: unknown; options: unknown }[] = [];
  const sign = ((
    _c: unknown,
    command: { constructor: { name: string }; input: unknown },
    options: unknown,
  ) => {
    signed.push({ command: command.constructor.name, input: command.input, options });
    return Promise.resolve('https://signed.example/x');
  }) as never;
  const store = s3FileStore({
    client: client as unknown as S3Client,
    bucket: 'b',
    now: () => NOW,
    sign,
  });
  return { store, calls, signed };
};

describe('s3-file-store v1', () => {
  it('presigns a PUT that signs the PDF content type and the exact length', async () => {
    const { store, signed } = setup();

    const result = await store.presignPut('docs/u1/a.pdf', 1234, 300);

    expect(result).toEqual({
      ok: true,
      value: { url: 'https://signed.example/x', expiresAt: '2026-11-12T00:05:00.000Z' },
    });
    expect(signed).toEqual([
      {
        command: 'PutObjectCommand',
        input: {
          Bucket: 'b',
          Key: 'docs/u1/a.pdf',
          ContentType: 'application/pdf',
          ContentLength: 1234,
        },
        options: { expiresIn: 300, signableHeaders: new Set(['content-type']) },
      },
    ]);
  });

  it('presigns a GET that serves application/pdf', async () => {
    const { store, signed } = setup();

    await store.presignGet('docs/u1/a.pdf', 900);

    expect(signed[0]).toMatchObject({
      command: 'GetObjectCommand',
      input: { Bucket: 'b', Key: 'docs/u1/a.pdf', ResponseContentType: 'application/pdf' },
      options: { expiresIn: 900 },
    });
  });

  it('reads the size, null for a missing object, UPSTREAM_UNAVAILABLE for other errors', async () => {
    const { store } = setup(
      { ContentLength: 42 },
      sdkError('NotFound', 404),
      sdkError('Forbidden', 403),
    );

    expect(await store.sizeOf('k')).toEqual({ ok: true, value: 42 });
    expect(await store.sizeOf('k')).toEqual({ ok: true, value: null });
    expect(await store.sizeOf('k')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE', details: { operation: 's3.HeadObject' } },
    });
  });

  it('reads the first bytes with a Range request', async () => {
    const body = { transformToByteArray: () => Promise.resolve(Buffer.from('%PDF-')) };
    const { store, calls } = setup({ Body: body }, {});

    expect(await store.firstBytes('k', 5)).toEqual({ ok: true, value: Buffer.from('%PDF-') });
    expect(await store.firstBytes('k', 5)).toEqual({ ok: true, value: new Uint8Array() });
    expect(calls[0]).toEqual({
      command: 'GetObjectCommand',
      input: { Bucket: 'b', Key: 'k', Range: 'bytes=0-4' },
    });
  });

  it('maps a signing failure to UPSTREAM_UNAVAILABLE', async () => {
    const { client } = stubClient();
    const sign = (() => Promise.reject(sdkError('CredentialsProviderError'))) as never;
    const store = s3FileStore({
      client: client as unknown as S3Client,
      bucket: 'b',
      now: () => NOW,
      sign,
    });

    expect(await store.presignGet('k', 1)).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });

  it('purges by ListObjectVersions pages and DeleteObject with VersionId for versions and delete markers', async () => {
    const { store, calls } = setup(
      {
        Versions: [{ Key: 'docs/u/a.pdf', VersionId: 'v1' }],
        DeleteMarkers: [{ Key: 'docs/u/a.pdf', VersionId: 'm1' }],
        IsTruncated: true,
        NextKeyMarker: 'docs/u/a.pdf',
        NextVersionIdMarker: 'm1',
      },
      {},
      {},
      { Versions: [{ Key: 'docs/u/a.pdf', VersionId: 'v0' }], IsTruncated: false },
      {},
    );

    expect(await store.purgePrefix('docs/u/a.pdf')).toEqual({ ok: true, value: 3 });
    expect(calls).toEqual([
      { command: 'ListObjectVersionsCommand', input: { Bucket: 'b', Prefix: 'docs/u/a.pdf' } },
      {
        command: 'DeleteObjectCommand',
        input: { Bucket: 'b', Key: 'docs/u/a.pdf', VersionId: 'v1' },
      },
      {
        command: 'DeleteObjectCommand',
        input: { Bucket: 'b', Key: 'docs/u/a.pdf', VersionId: 'm1' },
      },
      {
        command: 'ListObjectVersionsCommand',
        input: {
          Bucket: 'b',
          Prefix: 'docs/u/a.pdf',
          KeyMarker: 'docs/u/a.pdf',
          VersionIdMarker: 'm1',
        },
      },
      {
        command: 'DeleteObjectCommand',
        input: { Bucket: 'b', Key: 'docs/u/a.pdf', VersionId: 'v0' },
      },
    ]);
  });

  it('never has more than 8 DeleteObject calls in flight', async () => {
    const versions = Array.from({ length: 30 }, (_, i) => ({
      Key: 'docs/u/a.pdf',
      VersionId: `v${String(i)}`,
    }));
    let inFlight = 0;
    let peak = 0;
    const client = {
      send: async (command: { constructor: { name: string } }) => {
        if (command.constructor.name === 'ListObjectVersionsCommand')
          return { Versions: versions, IsTruncated: false };
        inFlight += 1;
        peak = Math.max(peak, inFlight);
        await new Promise((resolve) => setImmediate(resolve));
        inFlight -= 1;
        return {};
      },
    } as unknown as S3Client;

    const result = await s3FileStore({ client, bucket: 'b', now: () => NOW }).purgePrefix(
      'docs/u/a.pdf',
    );

    expect([result, peak]).toEqual([{ ok: true, value: 30 }, 8]);
  });

  it('returns a failed list or delete as UPSTREAM_UNAVAILABLE', async () => {
    const list = setup(sdkError('AccessDenied', 403));
    const del = setup(
      { Versions: [{ Key: 'k', VersionId: 'v' }], IsTruncated: false },
      sdkError('AccessDenied', 403),
    );

    expect(await list.store.purgePrefix('k')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE', details: { operation: 's3.ListObjectVersions' } },
    });
    expect(await del.store.purgePrefix('k')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE', details: { operation: 's3.DeleteObject' } },
    });
  });

  it('reads a whole object (missing is UPLOAD_INVALID) and puts a PDF part', async () => {
    const body = { transformToByteArray: () => Promise.resolve(Buffer.from('%PDF-x')) };
    const { store, calls } = setup({ Body: body }, sdkError('NoSuchKey', 404), {});

    expect(await store.read('docs/u/a.pdf')).toEqual({ ok: true, value: Buffer.from('%PDF-x') });
    expect(await store.read('docs/u/a.pdf')).toMatchObject({ error: { code: 'UPLOAD_INVALID' } });
    expect(await store.put('docs/u/a/part-1.pdf', Buffer.from('p'))).toEqual({
      ok: true,
      value: undefined,
    });
    expect(calls[0]).toEqual({
      command: 'GetObjectCommand',
      input: { Bucket: 'b', Key: 'docs/u/a.pdf' },
    });
    expect(calls[2]).toEqual({
      command: 'PutObjectCommand',
      input: {
        Bucket: 'b',
        Key: 'docs/u/a/part-1.pdf',
        Body: Buffer.from('p'),
        ContentType: 'application/pdf',
      },
    });
  });
});
