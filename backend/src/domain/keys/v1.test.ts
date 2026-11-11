import { describe, expect, it } from 'vitest';
import {
  chatGsiKey,
  chatKey,
  chatPk,
  chatsGsiPk,
  CHAT_SK_PREFIX,
  docKey,
  docObjectKey,
  DOC_SK_PREFIX,
  messageKey,
  MESSAGE_SK_PREFIX,
  ownerPk,
  totpKey,
  usageDayKey,
  usageTotalKey,
} from './v1.js';

describe('keys v1', () => {
  it('scopes doc, chat and usage items by the owner partition', () => {
    expect([docKey('kristoff', 'a1'), chatKey('kristoff', 'c1'), ownerPk('kristoff')]).toEqual([
      { PK: 'OWNER#kristoff', SK: 'DOC#a1' },
      { PK: 'OWNER#kristoff', SK: 'CHAT#c1' },
      'OWNER#kristoff',
    ]);
  });

  it('orders chats on GSI1 by update instant then id', () => {
    expect(chatGsiKey('kristoff', '2026-11-12T00:00:00.000Z', 'c1')).toEqual({
      GSI1PK: 'OWNER#kristoff#CHATS',
      GSI1SK: '2026-11-12T00:00:00.000Z#c1',
    });
    expect(chatsGsiPk('kristoff')).toBe('OWNER#kristoff#CHATS');
  });

  it('puts messages under their chat partition with a MSG# sort key', () => {
    expect([
      messageKey('c1', '01J'),
      chatPk('c1'),
      MESSAGE_SK_PREFIX,
      DOC_SK_PREFIX,
      CHAT_SK_PREFIX,
    ]).toEqual([{ PK: 'CHAT#c1', SK: 'MSG#01J' }, 'CHAT#c1', 'MSG#', 'DOC#', 'CHAT#']);
  });

  it('builds the usage day, usage total and TOTP keys', () => {
    expect([
      usageDayKey('kristoff', '2026-11-12'),
      usageTotalKey('kristoff'),
      totpKey('kristoff'),
    ]).toEqual([
      { PK: 'OWNER#kristoff', SK: 'USAGE#2026-11-12' },
      { PK: 'OWNER#kristoff', SK: 'USAGE#TOTAL' },
      { PK: 'AUTH#kristoff', SK: 'TOTP' },
    ]);
  });

  it('builds the S3 key from the owner and the doc id only', () => {
    expect(docObjectKey('kristoff', '0123456789abcdef')).toBe('docs/kristoff/0123456789abcdef.pdf');
  });
});
