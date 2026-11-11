// Runs the Dynamo-backed port contract suites against DynamoDB Local. Run with
// `DYNAMODB_LOCAL_URL=http://localhost:8000 npm run test:it` (vitest.it.config.ts); the default
// `npm test` excludes test/it/. Each suite gets a fresh table with the contract's key schema.
import { CreateTableCommand, DynamoDBClient } from '@aws-sdk/client-dynamodb';
import { DynamoDBDocumentClient } from '@aws-sdk/lib-dynamodb';
import { dynamoAuthStateStore } from '../../src/adapters/dynamo/dynamo-auth-state-store/index.js';
import { dynamoChatStore } from '../../src/adapters/dynamo/dynamo-chat-store/index.js';
import { dynamoDocStore } from '../../src/adapters/dynamo/dynamo-doc-store/index.js';
import { dynamoProjectStore } from '../../src/adapters/dynamo/dynamo-project-store/index.js';
import { dynamoUsageMeter } from '../../src/adapters/dynamo/dynamo-usage-meter/index.js';
import { authStateStoreContract } from '../support/contracts/auth-state-store.contract.js';
import { chatStoreContract } from '../support/contracts/chat-store.contract.js';
import { projectStoreContract } from '../support/contracts/project-store.contract.js';
import { docStoreContract } from '../support/contracts/doc-store.contract.js';
import { usageMeterContract } from '../support/contracts/usage-meter.contract.js';

const url = process.env.DYNAMODB_LOCAL_URL;
if (url === undefined || url === '') throw new Error('set DYNAMODB_LOCAL_URL to run test:it');

const raw = new DynamoDBClient({
  endpoint: url,
  region: 'ap-southeast-2',
  credentials: { accessKeyId: 'local', secretAccessKey: 'local' },
});
const client = DynamoDBDocumentClient.from(raw, {
  marshallOptions: { removeUndefinedValues: true },
});
let n = 0;
const freshTable = async () => {
  n += 1;
  const table = `kristoff-test-${String(Date.now())}-${String(n)}`;
  const s = { AttributeType: 'S' as const };
  await raw.send(
    new CreateTableCommand({
      TableName: table,
      BillingMode: 'PAY_PER_REQUEST',
      AttributeDefinitions: ['PK', 'SK', 'GSI1PK', 'GSI1SK'].map((AttributeName) => ({
        AttributeName,
        ...s,
      })),
      KeySchema: [
        { AttributeName: 'PK', KeyType: 'HASH' },
        { AttributeName: 'SK', KeyType: 'RANGE' },
      ],
      GlobalSecondaryIndexes: [
        {
          IndexName: 'GSI1',
          KeySchema: [
            { AttributeName: 'GSI1PK', KeyType: 'HASH' },
            { AttributeName: 'GSI1SK', KeyType: 'RANGE' },
          ],
          Projection: { ProjectionType: 'ALL' },
        },
      ],
    }),
  );
  return { client, table };
};
docStoreContract('dynamodb-local', async () => dynamoDocStore(await freshTable()));
chatStoreContract('dynamodb-local', async () => dynamoChatStore(await freshTable()));
authStateStoreContract('dynamodb-local', async () => dynamoAuthStateStore(await freshTable()));
usageMeterContract('dynamodb-local', async () => dynamoUsageMeter(await freshTable()));
projectStoreContract('dynamodb-local', async () => dynamoProjectStore(await freshTable()));
