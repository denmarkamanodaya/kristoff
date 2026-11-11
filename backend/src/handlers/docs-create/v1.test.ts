import { describe, expect, it } from 'vitest';
import { DocUploadResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsCreateHandler } from './v1.js';

const UPLOAD = {
  doc: aDoc({ status: 'pending' as const }),
  uploadUrl: 'https://put.example/x',
  uploadHeaders: { 'Content-Type': 'application/pdf' },
};

describe('docs-create handler v1', () => {
  it('passes the parsed body and answers the upload', async () => {
    const seen: unknown[] = [];
    const handler = createDocsCreateHandler({
      http: httpDeps(),
      createDocUpload: (p, input) => (seen.push([p, input]), Promise.resolve(ok(UPLOAD))),
    });

    const res = await handler(
      httpEvent({ routeKey: 'POST /v1/docs', body: { fileName: 'a.pdf', size: 10 } }),
    );

    expect([res.statusCode, DocUploadResponse.parse(bodyOf(res))]).toEqual([200, UPLOAD]);
    expect(seen).toEqual([[{ uid: 'kristoff' }, { fileName: 'a.pdf', size: 10 }]]);
  });

  it.each([
    ['size 0', { fileName: 'a.pdf', size: 0 }],
    ['size over 32 MiB', { fileName: 'a.pdf', size: 33_554_433 }],
    ['a 256-char name', { fileName: 'x'.repeat(256), size: 1 }],
  ])('answers 400 for %s', async (_label, body) => {
    const handler = createDocsCreateHandler({
      http: httpDeps(),
      createDocUpload: () => Promise.reject(new Error('called')),
    });

    expect((await handler(httpEvent({ routeKey: 'POST /v1/docs', body }))).statusCode).toBe(400);
  });
});
