// Runs every port contract suite against the in-memory fakes (the real Dynamo adapters run the
// same suites in dynamo-local.contract.test.ts when DYNAMODB_LOCAL_URL is set).
import { authStateStoreContract } from '../support/contracts/auth-state-store.contract.js';
import { chatStoreContract } from '../support/contracts/chat-store.contract.js';
import { docStoreContract } from '../support/contracts/doc-store.contract.js';
import { usageMeterContract } from '../support/contracts/usage-meter.contract.js';
import { memoryAuthStateStore } from '../support/fakes/memory-auth-state-store.js';
import { memoryChatStore } from '../support/fakes/memory-chat-store.js';
import { memoryDocStore } from '../support/fakes/memory-doc-store.js';
import { memoryUsageMeter } from '../support/fakes/memory-usage-meter.js';

docStoreContract('memory', () => Promise.resolve(memoryDocStore().store));
chatStoreContract('memory', () => Promise.resolve(memoryChatStore().store));
authStateStoreContract('memory', () => Promise.resolve(memoryAuthStateStore().store));
usageMeterContract('memory', () => Promise.resolve(memoryUsageMeter().meter));
