// Runs every port contract suite against the in-memory fakes (the real Dynamo adapters run the
// same suites in dynamo-local.contract.test.ts when DYNAMODB_LOCAL_URL is set).
import { authStateStoreContract } from '../support/contracts/auth-state-store.contract.js';
import { chatStoreContract } from '../support/contracts/chat-store.contract.js';
import { projectStoreContract } from '../support/contracts/project-store.contract.js';
import { fileStoreContract } from '../support/contracts/file-store.contract.js';
import { docStoreContract } from '../support/contracts/doc-store.contract.js';
import { usageMeterContract } from '../support/contracts/usage-meter.contract.js';
import { memoryAuthStateStore } from '../support/fakes/memory-auth-state-store.js';
import { memoryChatStore } from '../support/fakes/memory-chat-store.js';
import { memoryProjectStore } from '../support/fakes/memory-project-store.js';
import { memoryFileStore } from '../support/fakes/memory-file-store.js';
import { s3FileStore } from '../../src/adapters/s3-file-store/index.js';
import { s3MemoryClient } from '../support/s3-memory-client.js';
import { memoryDocStore } from '../support/fakes/memory-doc-store.js';
import { memoryUsageMeter } from '../support/fakes/memory-usage-meter.js';

docStoreContract('memory', () => Promise.resolve(memoryDocStore().store));
chatStoreContract('memory', () => Promise.resolve(memoryChatStore().store));
authStateStoreContract('memory', () => Promise.resolve(memoryAuthStateStore().store));
usageMeterContract('memory', () => Promise.resolve(memoryUsageMeter().meter));
projectStoreContract('memory', () => Promise.resolve(memoryProjectStore().store));
fileStoreContract('memory', () => {
  const fake = memoryFileStore();
  return { store: fake.store, upload: fake.put };
});
// The real adapter over a versioned in-memory S3 client (page size 2 forces ListObjectVersions
// pagination).
fileStoreContract('s3 adapter', () => {
  const s3 = s3MemoryClient(2);
  return { store: s3FileStore({ client: s3.client, bucket: 'b', now: () => 0 }), upload: s3.put };
});
